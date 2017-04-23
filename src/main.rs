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

use std::env;

use config::Config;
use errors::*;
use server::Server;

fn main() {
    let config_path = match env::args().nth(1) {
        Some(s) => s,
        None => "stunnel.toml".to_owned(),
    };
    let config = Config::from_file(config_path).unwrap();
    Server::from_config(config).start();
}
