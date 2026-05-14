use std::io::{Write, stdin, stdout};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};

use crate::{print_line, server::{data::Data, network_handler::NetworkHandler}};

pub struct Server {
    data: Data,
}

impl Server {
    pub fn new() -> Server {
        Server { data: Data::new() }
    }

    pub async fn run(&mut self) -> Result<(), String> {
        let network_handler = NetworkHandler::new(self.data.clone()).await;

        let mut message = String::new();

        enable_raw_mode().map_err(|e| e.to_string())?;

        crossterm::execute!(stdout(), cursor::MoveToColumn(0)).map_err(|e| e.to_string())?;

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

                            if self.process_command(message) {
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

    fn process_command(&mut self, command: String) -> bool {
        match command.as_str() {
            "help" => {
                print_line(format!("help                 - print help text"));
                print_line(format!("exit - close server"));
            }
            "exit" => {
                print_line(format!("Closing server!"));
            }
            _ => print_line(format!("Unknown command, type help for help")),
        }

        command == "exit"
    }
}
