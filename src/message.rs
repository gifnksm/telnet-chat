use std::net::SocketAddr;
use std::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub struct Member {
    pub name: String,
    pub addr: SocketAddr
}

pub struct Request {
    pub addr: SocketAddr,
    pub body: RequestBody
}

pub enum RequestBody {
    Join { tx: Sender<Notify> },
    Leave,
    List,
    Rename { name: String },
    Submit { message: String },
    UnicastMessage { message: String }
}

#[derive(Debug, Clone)]
pub enum Notify {
    Unicast(UnicastNotify),
    Broadcast(BroadcastNotify)
}

#[derive(Debug, Clone)]
pub enum UnicastNotify {
    Join { name: String },
    Leave,
    List(Vec<(Member)>),
    Rename(bool),
    Submit(bool),
    Message(String)
}

#[derive(Debug, Clone)]
pub enum BroadcastNotify {
    Join { name: String, addr: SocketAddr },
    Leave { name: String, addr: SocketAddr },
    Rename { old_name: String, new_name: String, addr: SocketAddr },
    Submit { name: String, addr: SocketAddr, message: String },
}
