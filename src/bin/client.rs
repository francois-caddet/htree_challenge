use clap::Parser;
use tokio::net;

#[derive(Parser)]
struct ClientArgs {
    file: String,
    server: String,
    #[arg(default_value_t = 2636)]
    port: u16,
}

#[tokio::main]
async fn main() {
    let args = ClientArgs::parse();
    let stream = net::TcpListener::bind((args.server, args.port))
        .await
        .unwrap();
    println!("Hello, world!");
}
