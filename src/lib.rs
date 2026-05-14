use std::io::stdout;

use crossterm::cursor;

pub mod client {
    pub mod client;
    pub mod data;
    mod network_handler;
    mod packet_handler;
}

pub mod server {
    pub mod data;
    mod network_handler;
    mod packet_handler;
    pub mod server;
}

pub mod constants;
pub mod encryption;
pub mod packet;

pub fn print_line(text: String) {
    println!("{}", text);
    crossterm::execute!(stdout(), cursor::MoveToColumn(0))
        .map_err(|e| e.to_string())
        .unwrap();
}
