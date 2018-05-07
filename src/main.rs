#![deny(warnings)]
#![feature(plugin)]
#![feature(custom_attribute)]
extern crate diwata_intel as intel;
extern crate diwata_server as server;
extern crate futures;
extern crate hyper;
extern crate rustorm;
extern crate serde;
extern crate serde_json;
extern crate structopt;
#[macro_use]
extern crate structopt_derive;
extern crate open;

use futures::Future;
use hyper::error::Error;
use hyper::header::ContentType;
use hyper::server::Http;
use hyper::server::Response;
use hyper::server::Service;
use hyper::Request;
use server::handler::Server;
use std::sync::{Arc, Mutex};
use structopt::StructOpt;

const HTML: &'static str = include_str!("../public/static/inline-cli.html");

#[derive(Clone)]
pub struct Instance {
    server: Arc<Mutex<Server>>,
}

impl Instance {
    pub fn new(server: Server) -> Instance {
        Instance {
            server: Arc::new(Mutex::new(server)),
        }
    }
}

impl Service for Instance {
    type Request = Request;
    type Response = Response;
    type Error = Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        let uri = req.uri().clone();
        let path = uri.path();
        let query = uri.query();

        let trim_path = path.trim_matches('/');
        let split_path: Vec<_> = trim_path.split('/').collect();
        let head = split_path[0];

        if head == "" {
            Box::new(futures::future::ok(handle_index(req)))
        } else {
            self.server.lock().unwrap().route(path, query, req)
        }
    }
}

fn handle_index(_req: Request) -> Response {
    let mut res = Response::new();
    res.headers_mut().set(ContentType::html());
    return res.with_body(HTML.as_bytes());
}

/// TODO: using a port that is already in used doesn't seem to error in hyper
fn run(ip: &str, port: u16) {
    let addr = format!("{}:{}", ip, port).parse().unwrap();
    let server = Server::new();
    let instance = Instance::new(server);
    let bind = Http::new().bind(&addr, move || Ok(instance.clone()));
    match bind {
        Ok(bind) => match bind.run() {
            Ok(_result) => println!("ok"),
            Err(e) => panic!("error: {:?}", e),
        },
        Err(e) => panic!("Error: {:?}", e),
    }
}

fn open_browser(uri: &str) {
    match open::that(uri) {
        Ok(_) => println!("browser launched"),
        Err(e) => println!("unable to open a browser {}", e),
    }
}

#[derive(StructOpt, Debug)]
#[structopt(name = "diwata", about = "A user friendly database interface")]
pub struct Opt {
    #[structopt(
        short = "u",
        long = "db-url",
        help = "Database url to connect to, when set all data is exposed without login needed in the client side"
    )]
    pub db_url: Option<String>,
    #[structopt(
        short = "a",
        long = "address",
        help = "The address the server would listen, default is 0.0.0.0",
        default_value = "0.0.0.0"
    )]
    pub address: String,
    #[structopt(
        short = "p",
        long = "port",
        help = "What port this server would listen to, default is 8000",
        default_value = "8000"
    )]
    pub port: u16,
    #[structopt(short = "o", long = "open", help = "open a browser")]
    pub open: bool,
}

fn main() {
    let opt = Opt::from_args();
    println!("opt: {:?}", opt);
    if let Some(db_url) = opt.db_url {
        match server::set_db_url(&db_url) {
            Ok(_) => println!("url is set"),
            Err(_) => println!("unable to set db_url"),
        }
    }
    let ip = opt.address;
    let address = match &*ip {
        "0.0.0.0" => "localhost",
        _ => &ip,
    };
    let port = opt.port;
    let uri = format!("http://{}:{}", address, port);
    println!("uri: {}", uri);
    if opt.open {
        open_browser(&uri);
    }
    run(&ip, port);
}
