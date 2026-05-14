use core::net;
use std::{
    io::{Write, read_to_string, stdin, stdout},
    net::SocketAddr,
};

use chacha20poly1305::{Key, KeyInit, XChaCha20Poly1305};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};

use crate::{
    client::{data::Data, network_handler::NetworkHandler},
    constants,
    encryption::{encrypt_message, generate_key},
    packet::Packet,
    print_line,
};

pub struct Client {
    network_handler: Option<NetworkHandler>,
    data: Data,
}

impl Client {
    pub fn new() -> Client {
        Client {
            network_handler: None,
            data: Data::new(),
        }
    }

    pub async fn run(&mut self) -> Result<(), String> {
        let mut message = String::new();

        enable_raw_mode().map_err(|e| e.to_string())?;

        loop {
            if event::poll(std::time::Duration::from_millis(500)).map_err(|e| e.to_string())? {
                if let Event::Key(key_event) = event::read().map_err(|e| e.to_string())? {
                    match key_event.code {
                        KeyCode::Char(c) => {
                            message.push(c);
                            print!("{}", c);
                            stdout().flush().map_err(|e| e.to_string())?;
                        }
                        KeyCode::Enter => {
                            println!();
                            crossterm::execute!(stdout(), cursor::MoveToColumn(0))
                                .map_err(|e| e.to_string())?;
                            if self.process_command(message).await {
                                break;
                            }

                            message = String::new();
                            crossterm::execute!(stdout(), cursor::MoveToColumn(0))
                                .map_err(|e| e.to_string())?;
                        }
                        KeyCode::Backspace => {
                            message.pop();
                            print!("\x08 \x08");
                            stdout().flush().map_err(|e| e.to_string())?;
                        }
                        _ => {}
                    }
                }
            }
        }

        disable_raw_mode().map_err(|e| e.to_string())?;

        Ok(())
    }

    async fn process_command(&mut self, command: String) -> bool {
        let split = command.split(" ").collect::<Vec<_>>();
        match split[0] {
            "help" => {
                print_line(format!("help                 - print help text"));
                print_line(format!("exit                 - close client"));
                print_line(format!("say <messsage>       - send a message"));
                print_line(format!("join <ip> <username> - attempt to join a server"));
                print_line(format!("setkey <key>         - set current master key"));
                print_line(format!("genkey <context>     - generate a master key"));
                print_line(format!("leave                - leave a server"));
            }
            "exit" => {
                print_line(format!("Closing client!"));
            }
            "say" => {
                if let Some(network_handler) = self.network_handler.as_ref() {
                    let cipher = self.data.cipher.read().await;
                    if let Some(cipher) = cipher.clone() {
                        let ciphertext = encrypt_message(
                            cipher,
                            *self.data.id.read().await,
                            *self.data.counter.read().await,
                            command[4..].as_bytes(),
                        );

                        network_handler.queue_tcp_packet(Packet::Chat {
                            counter: self.data.counter.read().await.clone(),
                            sender_id: self.data.id.read().await.clone(),
                            ciphertext,
                        });
                    } else {
                        print_line(format!("No cipher/master key :/"));
                    }
                } else {
                    print_line(format!("Not connected to server :/"));
                }
            }
            "join" => {
                if split.len() > 2 {
                    if self.network_handler.is_some() {
                        print_line(format!("You are already connected to a server"));
                    } else {
                        if let Err(e) = self
                            .connect(split[1].to_string(), split[2].to_string())
                            .await
                        {
                            print_line(format!("Error connecting: {}", e));
                        }
                    }
                } else {
                    print_line(format!("Invalid usage"));
                }
            }
            "leave" => {
                if let Some(network_handler) = self.network_handler.as_ref() {
                    network_handler.queue_tcp_packet(Packet::LeaveRequest);
                    network_handler.shutdown().await;
                    self.network_handler = None;
                } else {
                    print_line(format!("Not connected to server :/"));
                }
            }
            "setkey" => {
                if split.len() > 1 {
                    if split[1].len() == 64 {
                        let bytes = hex::decode(split[1]).unwrap();
                        let key: [u8; 32] = bytes.as_slice().try_into().unwrap();
                        *self.data.cipher.write().await =
                            Some(XChaCha20Poly1305::new(Key::from_slice(&key)));
                        print_line(format!("Successfully updated master key!"));
                    } else {
                        print_line(format!("Invalid key format, should be a 32 byte hex value"));
                    }
                } else {
                    print_line(format!("Invalid usage"));
                }
            }
            "genkey" => {
                if split.len() > 1 {
                    let key = generate_key(split[1]);
                    print_line(format!("Key: {}", hex::encode(key)));
                } else {
                    print_line(format!("Invalid usage"));
                }
            }
            _ => print_line(format!("Unknown command, type help for help")),
        }

        command == "exit"
    }

    async fn connect(&mut self, address: String, mut username: String) -> Result<(), String> {
        let (addr, port) = {
            let split = address.trim().split(":").collect::<Vec<_>>();

            if split.len() > 1 {
                (
                    split[0].to_string(),
                    split[1].parse::<u32>().map_err(|e| e.to_string())?,
                )
            } else {
                print_line(format!("Using default port {}", constants::SERVER_PORT));
                (address.trim().to_string(), constants::SERVER_PORT)
            }
        };

        username = username.trim().to_string();

        print_line(format!("Attempting to connect to {}:{}", addr, port));

        let network_handler = NetworkHandler::new(
            format!("{}:{}", addr, port)
                .trim()
                .parse::<SocketAddr>()
                .map_err(|e| e.to_string())?,
            username,
            self.data.clone(),
        )
        .await?;

        self.network_handler = Some(network_handler);

        Ok(())
    }
}
