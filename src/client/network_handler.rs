use crate::client::data::Data;
use crate::client::packet_handler;
use crate::constants;
use crate::packet::Packet;
use bincode::{config, decode_from_slice, encode_into_slice, encode_to_vec};
use futures::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpStream, UdpSocket};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{broadcast, mpsc};
use tokio::time::timeout;
use tokio_util::bytes::Bytes;
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

pub(in crate::client) struct NetworkHandler {
    tcp_tx: Arc<Sender<Packet>>,
    shutdown_tx: broadcast::Sender<()>,
    data: Data,
}

impl NetworkHandler {
    pub(in crate::client) async fn new(
        addr: SocketAddr,
        username: String,
        data: Data,
    ) -> Result<NetworkHandler, String> {
        let (tcp_tx, tcp_rx) = mpsc::channel::<Packet>(constants::CHANNEL_SIZE);
        let (shutdown_tx, _) = broadcast::channel::<()>(1);

        let tcp_tx = Arc::new(tcp_tx);

        let socket = Arc::new(
            UdpSocket::bind("0.0.0.0:0")
                .await
                .expect("Failed to bind UDP socket"),
        );

        let _ = socket.connect(addr).await;

        Self::start_tcp_client(
            tcp_tx.clone(),
            tcp_rx,
            addr.clone(),
            shutdown_tx.subscribe(),
            username,
            data.clone(),
        )
        .await?;

        Ok(NetworkHandler {
            tcp_tx,
            shutdown_tx,
            data,
        })
    }

    pub(in crate::client) fn queue_tcp_packet(&self, packet: Packet) {
        let tx = self.tcp_tx.clone();
        tokio::spawn(async move {
            tx.send(packet)
                .await
                .expect("Failed to send TCP packet over channel");
        });
    }

    pub(in crate::client) async fn shutdown(&self) {
        if let Err(e) = self.shutdown_tx.send(()) {
            eprintln!("Error sending shutdown signal: {:?}", e);
        }
    }

    async fn start_tcp_client(
        tx: Arc<Sender<Packet>>,
        rx: Receiver<Packet>,
        addr: SocketAddr,
        shutdown_rx: broadcast::Receiver<()>,
        username: String,
        data: Data,
    ) -> Result<(), String> {
        let stream = timeout(
            Duration::from_millis(constants::SERVER_CONNECT_TIMEOUT_MS),
            TcpStream::connect(addr),
        )
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

        let (reader, writer) = stream.into_split();

        let reader = FramedRead::new(reader, LengthDelimitedCodec::new());
        let writer = FramedWrite::new(writer, LengthDelimitedCodec::new());

        tokio::spawn(Self::start_tcp_reader(reader, tx, shutdown_rx, data));
        tokio::spawn(Self::start_tcp_writer(writer, rx, username));

        Ok(())
    }

    async fn start_tcp_reader(
        mut reader: FramedRead<OwnedReadHalf, LengthDelimitedCodec>,
        tx: Arc<Sender<Packet>>,
        mut shutdown_rx: broadcast::Receiver<()>,
        data: Data,
    ) {
        loop {
            tokio::select! {
                result = reader.next() => {
                    if let Some(Ok(bytes)) = result {
                        if let Ok((packet, _)) = decode_from_slice::<Packet, _>(&bytes, config::standard()) {
                            let response = packet_handler::handle_tcp_packet(packet, data.clone()).await;
                            for r in response {
                                if let Err(e) = tx.send(r).await {
                                    eprintln!("TCP: failed to send response over channel: {e}");
                                }
                            }
                        }
                    }
                }

                _ = shutdown_rx.recv() => {
                    break;
                }
            }
        }
    }

    async fn start_tcp_writer(
        mut writer: FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>,
        mut rx: Receiver<Packet>,
        username: String,
    ) {
        let bytes = encode_to_vec(Packet::JoinRequest { username }, config::standard()).unwrap();
        if let Err(e) = writer.send(Bytes::from(bytes)).await {
            eprintln!("TCP: failed to send init packet: {e}");
        }

        while let Some(packet) = rx.recv().await {
            let bytes = encode_to_vec(&packet, config::standard()).unwrap();
            if let Err(e) = writer.send(Bytes::from(bytes)).await {
                eprintln!("TCP: failed to send packet over network: {e}");
            }
        }
    }
}
