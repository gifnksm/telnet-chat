//! Telnet chat application

#![cfg_attr(not(test), deny(warnings))]

#![warn(missing_copy_implementations)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unused_extern_crates)]
#![warn(unused_import_braces)]
#![warn(unused_qualifications)]
#![warn(unused_results)]

#![feature(plugin)]
#![plugin(docopt_macros)]

extern crate docopt;
extern crate rand;
extern crate rustc_serialize;

use std::error::Error;
use std::io::{self, Write};
use std::process;

const DEFAULT_ADDR: &'static str = "localhost";
const DEFAULT_PORT: u16 = 10000;

mod error;
mod message;
mod server;
mod client;

docopt! {
    Args derive Debug, "
Usage: socket [options]
       socket --help

Options:
  --help        Show this message.
  --addr ADDR   IP address.
  --port PORT   Port number.
",
    flag_addr: Option<String>,
    flag_port: Option<u16>
}

fn main() {
    let args: Args = Args::docopt().decode().unwrap_or_else(|err| err.exit());

    let addr = args.flag_addr.as_ref().map(|s| s.as_ref()).unwrap_or(DEFAULT_ADDR);
    let port = args.flag_port.unwrap_or(DEFAULT_PORT);

    if let Err(err) = server::run(addr, port) {
        let _ = writeln!(io::stderr(), "{}", err);
        process::exit(1);
    }
}
