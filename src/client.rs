use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::TcpStream;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use error::AppResult;
use message::{Request, RequestBody, Notify, UnicastNotify, BroadcastNotify};

pub fn run(stream: TcpStream, req_tx: Sender<Request>) -> AppResult<()> {
    let cli_addr = try!(stream.peer_addr());

    let (ntf_tx, ntf_rx) = mpsc::channel();
    let req_body = RequestBody::Join { tx: ntf_tx };
    try!(req_tx.send(Request { addr: cli_addr, body: req_body }));

    let write_thread = {
        let stream = try!(stream.try_clone());
        thread::spawn(move || writer_loop(stream, ntf_rx))
    };

    let read_thread = {
        let req_tx = req_tx.clone();
        thread::spawn(move || reader_loop(stream, req_tx))
    };

    let _ = read_thread.join();
    let req_body = RequestBody::Leave;
    try!(req_tx.send(Request { addr: cli_addr, body: req_body }));
    let _ = write_thread.join();

    Ok(())
}

fn writer_loop(stream: TcpStream, ntf_rx: Receiver<Notify>) -> AppResult<()> {
    let mut writer = BufWriter::new(stream);

    try!(writeln!(writer, "Welcome to telnet chat!"));
    try!(writer.flush());

    let mut finished = false;
    while !finished {
        finished = match try!(ntf_rx.recv()) {
            Notify::Broadcast(bntf) => try!(handle_broadcast(&mut writer, bntf)),
            Notify::Unicast(untf) => try!(handle_unicast(&mut writer, untf))
        };
    }

    Ok(())
}

fn handle_broadcast(writer: &mut BufWriter<TcpStream>, ntf: BroadcastNotify) -> AppResult<bool> {
    match ntf {
        BroadcastNotify::Rename { old_name, new_name, .. } => {
            try!(writeln!(writer, "Rename: {} => {}", old_name, new_name));
            try!(writer.flush());
        }
        BroadcastNotify::Join { name, addr, .. } => {
            try!(writeln!(writer, "Join: {} from {}", name, addr));
            try!(writer.flush());
        }
        BroadcastNotify::Leave { name, .. } => {
            try!(writeln!(writer, "Leave: {}", name));
            try!(writer.flush());
        }
        BroadcastNotify::Submit { name, message, .. } => {
            try!(writeln!(writer, "{}: {}", name, message));
            try!(writer.flush());
        }
    }
    Ok(false)
}

fn handle_unicast(writer: &mut BufWriter<TcpStream>, ntf: UnicastNotify) -> AppResult<bool> {
    match ntf {
        UnicastNotify::Join { name } => {
            try!(writeln!(writer, "Hello! Your name is {}! Enjoy!", name));
            try!(writer.flush());
        }
        UnicastNotify::Leave => {
            return Ok(true);
        }
        UnicastNotify::List(list) => {
            try!(writeln!(writer, "Members:"));
            for mem in list {
                try!(writeln!(writer, "  {} from {}", mem.name, mem.addr));
            }
            try!(writer.flush());
        }
        UnicastNotify::Rename(succeed) => {
            if !succeed {
                try!(writeln!(writer, "=> rename failed"));
                try!(writer.flush());
            }
        }
        UnicastNotify::Submit(succeed) => {
            if !succeed {
                try!(writeln!(writer, "=> submitting message failed"));
                try!(writer.flush());
            }
        }
        UnicastNotify::Message(message) => {
            try!(writeln!(writer, "=> {}", message));
            try!(writer.flush());
        }
    }
    Ok(false)
}

fn reader_loop(stream: TcpStream, req_tx: Sender<Request>) -> AppResult<()> {
    let cli_addr = try!(stream.peer_addr());
    let mut reader = BufReader::new(try!(stream.try_clone()));

    let mut buf = String::new();
    loop {
        buf.clear();
        let _ = try!(reader.read_line(&mut buf));
        if buf.is_empty() { break }
        let input = buf.trim();
        if input.is_empty() { continue }

        let req_body = if !input.starts_with("\\") {
            RequestBody::Submit { message: input.to_string() }
        } else if input.starts_with("\\\\") {
            RequestBody::Submit { message: input[1..].to_string() }
        } else {
            fn is_whitespace(c: char) -> bool { c.is_whitespace() }
            let is_whitespace = is_whitespace;
            let mut it = input.splitn(2, is_whitespace);

            let command = it.next();
            let arg = it.next().map(|s| s.trim());
            match (command, arg) {
                (Some("\\exit"), None) => { break; }
                (Some("\\rename"), Some(name)) => {
                    let name = name.trim();
                    RequestBody::Rename { name: name.to_string() }
                }
                (Some("\\list"), None) => {
                    RequestBody::List
                }
                (Some("\\help"), None) => {
                    let message = format!("Commands:
  \\help            Show this message.
  \\exit            Exit from chat.
  \\list            Show participants list.
  \\rename NAME     Change your name.
");
                    RequestBody::UnicastMessage { message: message }
                }
                _ => {
                    let message = format!("Invalid command: {}", input);
                    RequestBody::UnicastMessage { message: message}
                }
            }
        };

        try!(req_tx.send(Request { addr: cli_addr, body: req_body }));
    }

    Ok(())
}
