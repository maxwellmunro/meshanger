use meSHAnger::{client::client::Client, constants};
use rand::Rng;

#[tokio::main]
async fn main() {

    let mut rng = rand::thread_rng();



    for line in constants::HELLO_MESSAGE {
        let line = line.to_string();

        let mut chars = line.chars().collect::<Vec<_>>();

        for i in 0..chars.len() {
            if chars[i] == '#' {
                chars[i] = if rng.gen_bool(0.5) { '1' } else { '0' };
            }
        }

        let line = chars.iter().collect::<String>();
        println!("{}", line);
    }

    println!();
    
    let mut client = Client::new();
    if let Err(e) = client.run().await {
        println!("Error occured :/\n{}", e);
    }
}
