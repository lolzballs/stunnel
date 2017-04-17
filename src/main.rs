extern crate bytes;
#[macro_use]
extern crate error_chain;
extern crate futures;
extern crate native_tls;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_tls;
extern crate toml;

mod config;
mod errors;
mod server;
mod tunnel;

use config::Config;
use errors::*;
use server::Server;

use std::net::ToSocketAddrs;

use futures::Stream;
use tokio_core::net::{TcpListener, TcpStream};
use tokio_core::reactor::Core;

fn main() {
    let config = Config::from_file("stunnel.toml").unwrap();
    println!("{:#?}", config);

    Server::from_config(config).start();
}
