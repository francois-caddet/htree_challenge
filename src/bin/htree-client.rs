use clap::{Parser, Subcommand};
use htree_challenge::tree::Proof;
use reqwest::blocking::{multipart::*, Client};
use serde_json::from_slice;
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
    let mut roots: HashMap<String, String> = roots_json
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
            let root = roots.get(&args.server);
            let res = if let Some(root) = root {
                req.query(&[("root", root)])
            } else {
                req
            }
            .send();
            let proof: Proof = res.unwrap().json().unwrap();
            let root = if proof.hash() == root.map(|r| blake3::Hash::from_hex(r).unwrap()) {
                println!("Uploaded ID: {}", proof.nth());
                proof.prove_on(hash)
            } else {
                panic!("Server corupted");
            };
            roots.insert(args.server, root.to_hex().to_string());
            fs::write("roots.json", serde_json::to_vec(&roots).unwrap()).unwrap();
        }
        Command::Get { nth, file } => {
            let root = &roots[&args.server];
            let res = client
                .get(format!("http://{}:{}/{}", args.server, args.port, nth))
                .query(&[("root", root)])
                .send()
                .unwrap();
            let bytes = res.bytes().unwrap();
            let res = client
                .get(format!(
                    "http://{}:{}/{}/proof",
                    args.server, args.port, nth
                ))
                .query(&[("root", root)])
                .send()
                .unwrap();
            let proof: Proof = res.json().unwrap();
            if proof
                .prove_on(blake3::hash(&bytes))
                .against(blake3::Hash::from_hex(root).unwrap())
            {
                fs::write(file.clone(), bytes).unwrap();
                println!("Downloaded file into: {}", file);
            } else {
                panic!("Server corupted");
            }
        }
        Command::Proof { nth, file } => {
            let root = &roots[&args.server];
            let bytes = fs::read(file.clone()).unwrap();
            let res = client
                .get(format!(
                    "http://{}:{}/{}/proof",
                    args.server, args.port, nth
                ))
                .query(&[("root", root)])
                .send()
                .unwrap();
            let proof: Proof = res.json().unwrap();
            if proof
                .prove_on(blake3::hash(&bytes))
                .against(blake3::Hash::from_hex(root).unwrap())
            {
                println!("Proved: {}", file);
            } else {
                panic!("Proof failed! you may try to provethe wrong file or may the server be corupted.");
            }
        }
    };
}
