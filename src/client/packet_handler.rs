use crate::{client::data::Data, encryption::decrypt_message, packet::Packet, print_line};

pub(in crate::client) async fn handle_tcp_packet(packet: Packet, data: Data) -> Vec<Packet> {
    match packet {
        Packet::Chat {
            counter,
            sender_id,
            ciphertext,
        } => {
            let cipher = data.cipher.read().await.clone();
            if let Some(cipher) = cipher {
                if let Ok(decrypted) =
                    decrypt_message(cipher, sender_id, counter, ciphertext.as_slice())
                {
                    let message = decrypted.into_iter().map(|c| c as char).collect::<String>();

                    if let Some(username) = data.users.read().await.get(&sender_id) {
                        print_line(format!("({}) {}: {}", counter, username, message));
                    } else {
                        print_line(format!("({}) unknown {}: {}", counter, sender_id, message));
                    }
                } else {
                    print_line(format!(
                        "Received message, unable to decryptm, you likely have the wrong key..."
                    ));
                }
            } else {
                print_line(format!("Received message, no cipher/master key found?"));
            }

            *data.counter.write().await += 1;
            vec![]
        }
        Packet::JoinResponseSuccess { id, counter, users } => {
            print_line(format!("Success joining server, id: {}", id));
            *data.id.write().await = id;
            *data.counter.write().await = counter;
            *data.users.write().await = users;
            vec![]
        }
        Packet::JoinResponseDeny { err } => {
            print_line(format!("Error joining server: {}", err));
            vec![]
        }
        Packet::UserLeft { id } => {
            if let Some(username) = data.users.write().await.remove(&id) {
                print_line(format!("User {} left", username));
            }
            data.users.write().await.remove(&id);
            vec![]
        }
        Packet::UserJoined { username } => {
            print_line(format!("I: User {} joined", username));
            data.users.write().await.insert(username);
            vec![]
        }
        Packet::Kick { reason } => {
            print_line(format!("You have been kicked for {}", reason));
            vec![]
        }
        _ => vec![],
    }
}
