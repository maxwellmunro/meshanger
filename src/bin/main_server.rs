use meSHAnger::server::server::Server;

#[tokio::main]
async fn main() {
    let mut server = Server::new();
    server.run().await;
}
