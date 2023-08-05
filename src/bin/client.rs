use clap::{Parser, Subcommand};
use reqwest::blocking::{multipart::*, Client};
use serde_json::{from_slice, to_vec};
use std::collections::HashMap;
use std::fs;

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

fn main() {
    let args = ClientArgs::parse();
    let roots_json = fs::read("roots.json");
    let roots: HashMap<String, String> = roots_json
        .map(|json| from_slice(&json).unwrap())
        .unwrap_or(HashMap::new());
    let client = Client::new();
    match args.cmd {
        Command::Push { file } => {
            let bytes = fs::read(file.clone()).unwrap();
            let hash = blake3::hash(&bytes);
            let req = client
                .post(format!("http://{}:{}", args.server, args.port))
                .multipart(
                    Form::new()
                        .text("hash", hash.to_hex().to_string())
                        .file("file", file)
                        .unwrap(),
                );
            println!("{:#?}", req);
            let res = if let Some(root) = roots.get(&args.server) {
                req.query(&("root", root))
            } else {
                req
            }
            .send();
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
