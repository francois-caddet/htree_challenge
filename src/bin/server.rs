use clap::Parser;
use htree_challenge::tree::*;
use salvo::fs::NamedFile;
use salvo::prelude::*;

use std::path::{PathBuf};
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
    let root = req.query::<String>("root");
    println!("load_store: {:?}", root);
    let path = root.map(|r| format!("data/{}.store", r));
    let store: HMap<String> = if let Some(p) = path {
        if fs::try_exists(&p).await.unwrap() {
            let data = fs::read(p).await.unwrap();
            serde_json::from_slice(&data).unwrap()
        } else {
            HMap::new()
        }
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
    println!("save_store");
    let root = depot.get::<blake3::Hash>("root").unwrap();
    let old_root = req.query::<String>("root");
    let path = format!("data/{}.store", root.to_hex());
    let store = depot.get::<HMap<String>>("store").unwrap();
    fs::write(path, serde_json::to_vec(store).unwrap())
        .await
        .unwrap();
    if let Some(old_path) = old_root.map(|r| format!("data/{}.store", r)) {
        fs::remove_file(old_path).await.unwrap();
    }
    fs::rename(
        depot.get::<PathBuf>("file").unwrap(),
        format!("data/{}", req.form::<String>("hash").await.unwrap()),
    )
    .await
    .unwrap();
}

#[handler]
async fn get(req: &mut Request, depot: &mut Depot, res: &mut Response, _ctrl: &mut FlowCtrl) {
    let store = depot.get::<HMap<String>>("store").unwrap();
    let id = req.param("id").unwrap();
    let name = store.get(id);
    if let Some(name) = name {
        NamedFile::builder(format!(
            "data/{}",
            store.get_hash(id).unwrap().to_hex().to_string()
        ))
        .attached_name(name)
        .send(req.headers(), res)
        .await;
    } else {
        res.render(StatusError::not_found());
    }
}

#[handler]
async fn get_proof(req: &mut Request, depot: &mut Depot, res: &mut Response, _ctrl: &mut FlowCtrl) {
    let store = depot.get::<HMap<String>>("store").unwrap();
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
    println!("push: {:#?}", hash);
    println!("push: {:#?}", req.form_data().await);
    let file = req.file("file").await.unwrap();
    {
        depot.insert("file", file.path().clone());
    }
    let store = depot.get_mut::<HMap<String>>("store").unwrap();
    let proof = store.push(hash, file.name().unwrap().to_string());
    let root: blake3::Hash = *proof.prove_on(hash);
    depot.insert("root", root);
    res.render(Json(proof));
}

#[tokio::main]
async fn main() {
    if !fs::try_exists("data").await.unwrap() {
        fs::create_dir("data").await.unwrap();
    }
    let args = ServerArgs::parse();
    let acceptor = TcpListener::new((args.server, args.port)).bind().await;
    let router = Router::with_hoop(load_store)
        .push(Router::with_hoop(push).post(save_store))
        .push(
            Router::with_path("<id: num>")
                .get(get)
                .push(Router::with_path("proof").get(get_proof)),
        );
    println!("{:#?}", router);
    Server::new(acceptor).serve(router).await;
}
