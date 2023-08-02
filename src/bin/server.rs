use clap::Parser;
use tokio::net;

#[derive(Parser)]
struct ServerArgs {
    #[arg(default_value_t=String::from("127.0.0.1"))]
    server: String,
    #[arg(default_value_t = 2636)]
    port: u16,
}

#[tokio::main]
async fn main() {
    let args = ServerArgs::parse();
    let stream = net::TcpStream::connect((args.server, args.port))
        .await
        .unwrap();
    println!("Hello, world!");
}
