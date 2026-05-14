use crate::packet::Packet;
use crate::print_line;
use crate::server::data::Data;
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug)]
pub(in crate::server) enum Response {
    Broadcast {
        packet: Packet,
        sender_addr: Option<SocketAddr>,
    },
    Reply {
        packet: Packet,
        addr: SocketAddr,
    },
}

pub(in crate::server) async fn handle_tcp_packet(
    packet: Packet,
    id: u64,
    addr: SocketAddr,
    tcp_addresses: Arc<RwLock<HashSet<SocketAddr>>>,
    data: Data,
) -> Vec<Response> {
    match packet {
        Packet::JoinRequest { username } => {
            if data.users.read().await.iter().any(|(_, u)| *u == username) {
                return vec![Response::Reply {
                    packet: Packet::JoinResponseDeny {
                        err: String::from(
                            "A client with the same username is already in the room :/",
                        ),
                    },
                    addr,
                }];
            }

            let id = data.users.write().await.insert(username.clone());

            tcp_addresses.write().await.insert(addr);

            print_line(format!(
                "User {} joined from {}, id: {}",
                username, addr, id
            ));

            vec![
                Response::Reply {
                    packet: Packet::JoinResponseSuccess {
                        id,
                        counter: *data.counter.read().await,
                        users: data.users.read().await.clone(),
                    },
                    addr,
                },
                Response::Broadcast {
                    packet: Packet::UserJoined { username },
                    sender_addr: Some(addr),
                },
            ]
        }
        Packet::Chat {
            counter,
            sender_id,
            ciphertext,
        } => {
            print_line(format!(
                "C: {}, SID: {}, cipher: {:?}",
                counter, sender_id, ciphertext
            ));

            *data.counter.write().await = counter;

            vec![Response::Broadcast {
                packet: Packet::Chat {
                    counter,
                    sender_id,
                    ciphertext,
                },
                sender_addr: None,
            }]
        }
        Packet::InternalClientDisconnect | Packet::LeaveRequest => {
            let mut users = data.users.write().await;

            let Some(username) = users.get(&id) else {
                return vec![];
            };

            print_line(format!("Disconnecting client {} AKA {}", addr, username));

            users.remove(&id);

            vec![Response::Broadcast {
                packet: Packet::UserLeft { id },
                sender_addr: Some(addr),
            }]
        }
        _ => vec![],
    }
}
