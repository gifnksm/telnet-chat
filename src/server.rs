extern crate rand;

use std::collections::{HashMap, HashSet};
use std::io::{self, Write};
use std::mem;
use std::net::{SocketAddr, TcpListener};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use rand::Rng;

use error::AppResult;
use message::{Member, Request, RequestBody, Notify, UnicastNotify, BroadcastNotify};
use client;

struct Client {
    addr: SocketAddr,
    name: String,
    tx: Sender<Notify>
}

impl Client {
    fn join(&self) -> AppResult<()> {
        let ntf = UnicastNotify::Join { name: self.name.clone() };
        try!(self.tx.send(Notify::Unicast(ntf)));
        Ok(())
    }
    fn leave(self) -> AppResult<()> {
        let rsp = UnicastNotify::Leave;
        try!(self.tx.send(Notify::Unicast(rsp)));
        Ok(())
    }
    fn list(&self, list: Vec<Member>) -> AppResult<()> {
        let rsp = UnicastNotify::List(list);
        try!(self.tx.send(Notify::Unicast(rsp)));
        Ok(())
    }
    fn rename_success(&mut self, name: String) -> AppResult<String> {
        try!(self.send_rename(true));
        Ok(mem::replace(&mut self.name, name))
    }
    fn rename_failure(&self) -> AppResult<()> {
        self.send_rename(false)
    }
    fn submit_success(&self) -> AppResult<()> {
        self.send_submit(true)
    }
    // fn submit_failure(&self) -> AppResult<()> {
    //     self.send_submit(false)
    // }
    fn message(&self, message: String) -> AppResult<()> {
        let rsp = UnicastNotify::Message(message);
        try!(self.tx.send(Notify::Unicast(rsp)));
        Ok(())
    }

    fn send_rename(&self, succeed: bool) -> AppResult<()> {
        let rsp = UnicastNotify::Rename(succeed);
        try!(self.tx.send(Notify::Unicast(rsp)));
        Ok(())
    }
    fn send_submit(&self, succeed: bool) -> AppResult<()> {
        let rsp = UnicastNotify::Submit(succeed);
        try!(self.tx.send(Notify::Unicast(rsp)));
        Ok(())
    }
}

fn gen_random_name<R: Rng>(rng: &mut R) -> String {
    const CHARSET: &'static [u8] = b"0123456789";

    let mut name = "anonymous-".to_string();
    for _ in 0..5 {
        name.push(*rng.choose(CHARSET).unwrap() as char);
    }
    name
}

pub fn run(addr: &str, port: u16) -> AppResult<()> {
    let listener = try!(TcpListener::bind((addr, port)));
    let local_addr = try!(listener.local_addr());
    println!("Listening {}", local_addr);

    let (req_tx, req_rx) = mpsc::channel();

    let _ = thread::spawn(move || server_loop(req_rx));

    for stream in listener.incoming() {
        match stream {
            Err(err) => {
                let _ = writeln!(io::stderr(), "{}", err);
                continue
            }
            Ok(stream) => {
                let cli_addr = try!(stream.peer_addr());
                println!("Connected from {}", cli_addr);

                let req_tx = req_tx.clone();
                let _ = thread::spawn(move || {
                    if let Err(err) = client::run(stream, req_tx) {
                        let _ = writeln!(io::stderr(), "Client {} aborted: {}",
                                         cli_addr, err);
                    }
                });
            }
        }
    }

    return Ok(());
}

fn server_loop(req_rx: Receiver<Request>) {
    let mut rng = rand::thread_rng();
    let mut map = HashMap::new();
    let mut names = HashSet::new();

    loop {
        let mut broadcast = None;

        match req_rx.recv() {
            Err(err) => {
                let _ = writeln!(io::stderr(), "Request recv error: {}", err);
            }
            Ok(req) => {
                let addr = req.addr;
                if let Err(err) = handle_request(req, &mut map, &mut names, &mut broadcast, &mut rng) {
                    let _ = writeln!(io::stderr(), "Response send Rrror: {}", err);
                    if let Some(cli) = map.remove(&addr) {
                        let _ = cli.leave();
                    }
                }
            }
        };

        if let Some(ntf) = broadcast {
            for cli in map.values() {
                let _ = cli.tx.send(Notify::Broadcast(ntf.clone()));
            }
        }
    }
}

fn handle_request<R: Rng>(
    Request {addr, body}: Request,
    map: &mut HashMap<SocketAddr, Client>,
    names: &mut HashSet<String>,
    broadcast: &mut Option<BroadcastNotify>,
    rng: &mut R)
    -> AppResult<()>
{
    match body {
        RequestBody::Join { tx } => {
            let mut name;
            loop {
                name = gen_random_name(rng);
                if !names.contains(&name) {
                    break;
                }
            }

            let cli = Client { name: name.clone(), addr: addr, tx: tx };
            try!(cli.join());

            names.insert(cli.name.clone());
            let _ = map.insert(addr, cli);
            *broadcast = Some(BroadcastNotify::Join { name: name, addr: addr });
        }
        RequestBody::Leave => {
            if let Some(cli) = map.remove(&addr) {
                let name = cli.name.clone();
                let addr = cli.addr;
                if let Err(err) = cli.leave() {
                    let _ = writeln!(io::stderr(), "Response send Rrror: {}", err);
                    // continue if fails
                }

                *broadcast = Some(BroadcastNotify::Leave { name: name, addr: addr });
            }
        }
        RequestBody::List => {
            if let Some(cli) = map.get(&addr) {
                let list = map.values().map(|cli| {
                    Member {
                        name: cli.name.clone(),
                        addr: cli.addr
                    }
                }).collect();
                try!(cli.list(list));
            }
        }
        RequestBody::Rename { name } => {
            if let Some(cli) = map.get_mut(&addr) {
                let can_change = !names.contains(&name);

                if can_change {
                    let old_name = try!(cli.rename_success(name.clone()));
                    names.remove(&old_name);
                    names.insert(name.clone());
                    *broadcast = Some(BroadcastNotify::Rename {
                        old_name: old_name,
                        new_name: name,
                        addr: cli.addr
                    });
                } else {
                    try!(cli.rename_failure());
                }
            }
        }
        RequestBody::Submit { message } => {
            if let Some(cli) = map.get(&addr) {
                try!(cli.submit_success());

                *broadcast = Some(BroadcastNotify::Submit {
                    name: cli.name.clone(),
                    addr: cli.addr,
                    message: message
                });
            }
        }
        RequestBody::UnicastMessage { message } => {
            if let Some(cli) = map.get(&addr) {
                try!(cli.message(message));
            }
        }
    }
    Ok(())
}
