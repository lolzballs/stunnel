extern crate bytes;
extern crate futures;
extern crate native_tls;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_tls;

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
    let mut core = Core::new().expect("could not create reactor core");
    let handle = core.handle();

    let listen_addr = "127.0.0.1:1194".parse().unwrap();
    let listener = TcpListener::bind(&listen_addr, &handle).unwrap();

    let remote_addr = "ayy.bcheng.cf:443"
        .to_socket_addrs()
        .unwrap()
        .next()
        .unwrap();

    let server = listener
        .incoming()
        .for_each(|(local_sock, _)| {
            let local_addr = local_sock.peer_addr().unwrap();
            println!("recieved connection from {}", local_addr);
            let local_read = TunnelStream(Arc::new(local_sock));
            let local_write = local_read.clone();

            let cx = TlsConnector::builder().unwrap().build().unwrap();
            let remote_sock = TcpStream::connect(&remote_addr, &handle);

            let tls_handshake = {
                let local_addr = local_addr.clone();
                remote_sock.and_then(move |socket| {
                                         let tls = cx.connect_async("ayy.bcheng.cf", socket);
                                         tls.map_err(|e| io::Error::new(io::ErrorKind::Other, e))
                                     })
            };

            let tunnel = {
                let local_addr = local_addr.clone();
                let remote_addr = remote_addr.clone();
                tls_handshake.and_then(move |socket| {
                    println!("[{}]: started tunneling to {}", local_addr, remote_addr);
                    let (remote_read, remote_write) = socket.split();

                    let to_server =
                        copy(local_read, remote_write).map(|(n, _, writer)| shutdown(writer));
                    let to_client =
                        copy(remote_read, local_write).map(|(n, _, writer)| shutdown(writer));

                    to_server.join(to_client)
                })
            };

            let msg = {
                let local_addr = local_addr.clone();
                tunnel
                    .map(move |(from_client, from_server)| {
                             println!("[{}]: client disconnected", local_addr);
                         })
                    .map_err(move |e| {
                                 // Don't panic. Maybe the client just disconnected too soon.
                                 println!("[{}]: error: {}", local_addr, e);
                             })
            };

            handle.spawn(msg);

            Ok(())
        });

    core.run(server).unwrap();
}

#[derive(Clone)]
struct TunnelStream(Arc<TcpStream>);

impl Read for TunnelStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        (&*self.0).read(buf)
    }
}

impl Write for TunnelStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        (&*self.0).write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl AsyncRead for TunnelStream {}
impl AsyncWrite for TunnelStream {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        try!(self.0.shutdown(Shutdown::Write));
        Ok(().into())
    }
}
