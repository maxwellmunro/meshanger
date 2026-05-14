use crate::constants;
use crate::packet::Packet;
use crate::server::data::Data;
use crate::server::packet_handler;
use crate::server::packet_handler::Response;
use bincode::{config, decode_from_slice, encode_to_vec};
use futures::{SinkExt, StreamExt};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{RwLock, mpsc};
use tokio_util::bytes::Bytes;
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

pub(in crate::server) struct NetworkHandler {
    writers: Arc<
        RwLock<HashMap<SocketAddr, Arc<RwLock<FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>>>>>,
    >,
    tcp_addresses: Arc<RwLock<HashSet<SocketAddr>>>,
}

impl NetworkHandler {
    pub(in crate::server) async fn new(data: Data) -> NetworkHandler {
        let tcp_addresses = Arc::new(RwLock::new(HashSet::<SocketAddr>::new()));

        let writers = Arc::new(RwLock::new(HashMap::<
            SocketAddr,
            Arc<RwLock<FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>>>,
        >::new()));

        tokio::spawn(Self::start_tcp_server(
            tcp_addresses.clone(),
            writers.clone(),
            data,
        ));

        NetworkHandler {
            writers,
            tcp_addresses,
        }
    }

    pub(in crate::server) fn queue_tcp_response(&self, response: Response) {
        tokio::spawn(Self::handle_tcp_response(
            response,
            self.writers.clone(),
            self.tcp_addresses.clone(),
        ));
    }

    async fn start_tcp_server(
        tcp_addresses: Arc<RwLock<HashSet<SocketAddr>>>,
        writers: Arc<
            RwLock<
                HashMap<SocketAddr, Arc<RwLock<FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>>>>,
            >,
        >,
        data: Data,
    ) {
        let socket = TcpListener::bind(format!("0.0.0.0:{}", constants::SERVER_PORT))
            .await
            .unwrap();

        let mut id: u64 = 0;

        loop {
            if let Ok((stream, addr)) = socket.accept().await {
                Self::handle_tcp_client(
                    tcp_addresses.clone(),
                    writers.clone(),
                    stream,
                    addr,
                    id,
                    data.clone(),
                )
                .await;
                id += 1;
            }
        }
    }

    async fn handle_tcp_client(
        tcp_addresses: Arc<RwLock<HashSet<SocketAddr>>>,
        writers: Arc<
            RwLock<
                HashMap<SocketAddr, Arc<RwLock<FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>>>>,
            >,
        >,
        stream: TcpStream,
        addr: SocketAddr,
        id: u64,
        data: Data,
    ) {
        let (reader, writer) = stream.into_split();

        let reader = FramedRead::new(reader, LengthDelimitedCodec::new());
        let writer = FramedWrite::new(writer, LengthDelimitedCodec::new());

        writers
            .write()
            .await
            .insert(addr, Arc::new(RwLock::new(writer)));

        let (tx, rx) = mpsc::channel::<Response>(constants::CHANNEL_SIZE);

        tokio::spawn(Self::start_tcp_reader(
            tcp_addresses.clone(),
            writers.clone(),
            reader,
            tx,
            addr,
            id,
            data,
        ));
        tokio::spawn(Self::start_tcp_writer(tcp_addresses, rx, writers));
    }

    async fn start_tcp_reader(
        tcp_addresses: Arc<RwLock<HashSet<SocketAddr>>>,
        writers: Arc<
            RwLock<
                HashMap<SocketAddr, Arc<RwLock<FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>>>>,
            >,
        >,
        mut reader: FramedRead<OwnedReadHalf, LengthDelimitedCodec>,
        tx: Sender<Response>,
        addr: SocketAddr,
        id: u64,
        data: Data,
    ) {
        while let Some(Ok(bytes)) = reader.next().await {
            if let Ok((packet, _)) = decode_from_slice::<Packet, _>(&bytes, config::standard()) {
                let responses = packet_handler::handle_tcp_packet(
                    packet,
                    id,
                    addr,
                    tcp_addresses.clone(),
                    data.clone(),
                )
                .await;
                for r in responses {
                    if let Err(e) = tx.send(r).await {
                        eprintln!("TCP: failed to send response over channel: {e}");
                    }
                }
            }
        }

        tcp_addresses.write().await.remove(&addr);
        writers.write().await.remove(&addr);

        let responses = packet_handler::handle_tcp_packet(
            Packet::InternalClientDisconnect,
            id,
            addr,
            tcp_addresses.clone(),
            data,
        )
        .await;

        for r in responses {
            if let Err(e) = tx.send(r).await {
                eprintln!("TCP: failed to send response over channel: {e}");
            }
        }
    }

    async fn start_tcp_writer(
        addresses: Arc<RwLock<HashSet<SocketAddr>>>,
        mut rx: Receiver<Response>,
        writers: Arc<
            RwLock<
                HashMap<SocketAddr, Arc<RwLock<FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>>>>,
            >,
        >,
    ) {
        while let Some(response) = rx.recv().await {
            Self::handle_tcp_response(response, writers.clone(), addresses.clone()).await;
        }
    }

    async fn handle_tcp_response(
        response: Response,
        writers: Arc<
            RwLock<
                HashMap<SocketAddr, Arc<RwLock<FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>>>>,
            >,
        >,
        addresses: Arc<RwLock<HashSet<SocketAddr>>>,
    ) {
        match response {
            Response::Broadcast {
                packet,
                sender_addr,
            } => {
                if let Ok(buf) = encode_to_vec(packet.clone(), config::standard()) {
                    let writers_to_map = writers.read().await;
                    let writers_to_send = addresses
                        .read()
                        .await
                        .iter()
                        .filter(|addr| {
                            if let Some(sender_addr) = sender_addr {
                                *addr != &sender_addr
                            } else {
                                true
                            }
                        })
                        .map(|w| writers_to_map.get(w))
                        .flatten()
                        .collect::<Vec<_>>();

                    for writer in writers_to_send {
                        if let Err(e) = writer.write().await.send(Bytes::from(buf.clone())).await {
                            eprintln!("TCP: failed to send packet over channel: {e}");
                        }
                    }
                }
            }
            Response::Reply { packet, addr } => {
                if let Ok(buf) = encode_to_vec(packet, config::standard()) {
                    if let Some(writer) = writers.read().await.get(&addr) {
                        if let Err(e) = writer.write().await.send(Bytes::from(buf)).await {
                            eprintln!("TCP: failed to send packet over channel: {e}");
                        }
                    }
                }
            }
        }
    }
}
