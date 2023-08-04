use clap::Parser;
use htree_challenge::tree::*;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;

#[derive(Parser)]
struct ServerArgs {
    #[arg(default_value_t=String::from("127.0.0.1"))]
    server: String,
    #[arg(default_value_t = 2636)]
    port: u16,
}

#[handler]
async fn load_store(
    req: &mut Request,
    depot: &mut Depot,
    _res: &mut Response,
    _ctrl: &mut FlowCtrl,
) {
    let root = req.query::<String>("root").unwrap();
    let path = format!("data/{}.json", root);
    let store: HMap<PathBuf> = if fs::try_exists(&path).await.unwrap() {
        let data = fs::read(path).await.unwrap();
        serde_json::from_slice(&data).unwrap()
    } else {
        HMap::new()
    };
    depot.insert("store", store);
}

#[handler]
async fn save_store(
    req: &mut Request,
    depot: &mut Depot,
    _res: &mut Response,
    _ctrl: &mut FlowCtrl,
) {
    let root = depot.get::<blake3::Hash>("root").unwrap();
    let old_root = req.query::<String>("root");
    let path = format!("data/{}.json", root.to_hex());
    let store = depot.get::<HMap<PathBuf>>("store").unwrap();
    fs::write(path, serde_json::to_vec(store).unwrap())
        .await
        .unwrap();
    if let Some(old_path) = old_root.map(|r| format!("data/{}.json", r)) {
        fs::remove_file(old_path).await.unwrap();
    }
}

#[handler]
async fn get(req: &mut Request, depot: &mut Depot, res: &mut Response, _ctrl: &mut FlowCtrl) {
    let store = depot.get::<HMap<PathBuf>>("store").unwrap();
    let id = req.param("id").unwrap();
    let ret = store.get(id);
    if let Some((proof, path)) = ret {
        res.render(Json((proof, path)));
    } else {
        res.render(StatusError::not_found());
    }
}

#[handler]
async fn get_proof(req: &mut Request, depot: &mut Depot, res: &mut Response, _ctrl: &mut FlowCtrl) {
    let store = depot.get::<HMap<PathBuf>>("store").unwrap();
    let id = req.param("id").unwrap();
    let ret = store.proof(id);
    if let Some(proof) = ret {
        res.render(Json(proof));
    } else {
        res.render(StatusError::not_found());
    }
}

#[handler]
async fn push(req: &mut Request, depot: &mut Depot, res: &mut Response, _ctrl: &mut FlowCtrl) {
    let hash = blake3::Hash::from_hex(req.form::<String>("hash").await.unwrap()).unwrap();
    let file = req.file("file").await.unwrap();
    let store = depot.get_mut::<HMap<PathBuf>>("store").unwrap();
    let proof = store.push(hash, file.name().unwrap().into());
    let root: blake3::Hash = *proof.prove_on(hash);
    depot.insert("root", root);
    res.render(Json(proof));
}

#[tokio::main]
async fn main() {
    let args = ServerArgs::parse();
    let acceptor = TcpListener::new((args.server, args.port)).bind().await;
    let router = Router::with_hoop(load_store)
        .push(Router::new().post(push).hoop(save_store))
        .push(
            Router::with_path("<id: num>")
                .get(get)
                .push(Router::with_path("proof").get(get_proof)),
        );
    Server::new(acceptor).serve(router).await;
}
