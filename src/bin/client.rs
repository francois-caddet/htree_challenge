use clap::{Parser, Subcommand};
use reqwest::{multipart::*, Client};
use serde_json::{from_slice, to_vec};
use std::collections::HashMap;
use tokio::fs;

#[derive(Subcommand)]
enum Command {
    Get { nth: usize, file: String },
    Proof { nth: usize, file: String },
    Push { file: String },
}

#[derive(Parser)]
struct ClientArgs {
    #[command(subcommand)]
    cmd: Command,
    server: String,
    #[arg(default_value_t = 2636)]
    port: u16,
}

#[tokio::main]
async fn main() {
    let args = ClientArgs::parse();
    let roots_json = fs::read("roots.json").await;
    let roots: HashMap<String, String> = roots_json
        .map(|json| from_slice(&json).unwrap())
        .unwrap_or(HashMap::new());
    let client = Client::new();
    match args.cmd {
        Command::Push { file } => {
            let bytes = fs::read(file.clone()).await.unwrap();
            let hash = blake3::hash(&bytes);
            let req = client
                .post(format!("http://{}:{}", args.server, args.port))
                .multipart(
                    Form::new()
                        .text("hash", hash.to_hex().to_string())
                        .part("file", Part::bytes(bytes).file_name(file)),
                );
            let res = if let Some(root) = roots.get(&args.server) {
                req.query(&("root", root))
            } else {
                req
            }
            .send()
            .await;
            println!("{:#?}", res);
        }
        Command::Get { nth, file } => {
            todo!();
        }
        Command::Proof { file, nth } => {
            todo!();
        }
    };
}
