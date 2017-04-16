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

mod errors;
mod config;
mod tunnel;

use config::Config;
use errors::*;

use std::io::{self, Read, Write};
use std::net::{Shutdown, ToSocketAddrs};
use std::sync::Arc;

use futures::{Future, Stream, Poll};
use native_tls::TlsConnector;
use tokio_core::net::{TcpListener, TcpStream};
use tokio_core::reactor::Core;
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_io::io::{copy, shutdown};
use tokio_tls::TlsConnectorExt;

fn main() {
    let config = Config::from_file("stunnel.toml").unwrap();
    println!("{:#?}", config);
    let mut core = Core::new().expect("could not create reactor core");
    let handle = core.handle();

    let listen_addr = config.listen.parse().unwrap();
    let listener = TcpListener::bind(&listen_addr, &handle).unwrap();
    println!("Listening on: {}", listen_addr);

    let remote_addr = config
        .remote
        .to_socket_addrs()
        .unwrap()
        .next()
        .unwrap();

    let server = listener
        .incoming()
        .for_each(|(local_sock, _)| {
            let local_addr = local_sock.peer_addr().unwrap();
            println!("recieved connection from {}", local_addr);
            let remote_sock = TcpStream::connect(&remote_addr, &handle);

            let sni_addr = match config.sni_addr {
                Some(ref s) => s.clone(),
                None => config.remote.split(':').nth(0).unwrap().into(),
            };

            tunnel::start_tunnel(&handle, local_sock, remote_sock, sni_addr);
            Ok(())
        });

    core.run(server).unwrap();
}
