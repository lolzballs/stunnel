use server::Tunnel;

use std::io::{self, Read, Write};
use std::net::Shutdown;
use std::rc::Rc;
use std::sync::Arc;

use futures::{Future, Poll};
use native_tls::TlsConnector;
use tokio_core::reactor::Handle;
use tokio_core::net::{TcpStream, TcpStreamNew};
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_io::io::{copy, shutdown};
use tokio_tls::TlsConnectorExt;

pub fn start_tunnel(handle: &Handle,
                    tunnel: Rc<Tunnel>,
                    local_sock: TcpStream,
                    remote_sock: TcpStreamNew) {
    let local_addr = local_sock.peer_addr().unwrap();
    let local_read = TunnelStream(Arc::new(local_sock));
    let local_write = local_read.clone();

    let cx = tunnel.connector.clone();
    let tls_handshake = {
        let tunnel = tunnel.clone();
        let local_addr = local_addr.clone();
        remote_sock.and_then(move |socket| {
            println!("[{} {}]: starting TLS connection to {}, with SNI address {}",
                     tunnel.name,
                     local_addr,
                     tunnel.remote,
                     tunnel.sni_addr);
            cx.connect_async(&tunnel.sni_addr, socket)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
        })
    };
    let tunnel_future = {
        let tunnel = tunnel.clone();
        let local_addr = local_addr.clone();
        tls_handshake.and_then(move |socket| {
            println!("[{} {}]: started tunneling to {}",
                     tunnel.name,
                     local_addr,
                     tunnel.remote);
            let (remote_read, remote_write) = socket.split();
            let to_server = copy(local_read, remote_write).map(|(_, _, writer)| shutdown(writer));
            let to_client = copy(remote_read, local_write).map(|(_, _, writer)| shutdown(writer));
            to_server.join(to_client)
        })
    };
    let msg = {
        let tunnel2 = tunnel.clone();
        tunnel_future
            .map(move |_| {
                     println!("[{} {}]: client disconnected", tunnel.name, local_addr);
                 })
            .map_err(move |e| {
                         println!("[{} {}]: error: {}", tunnel2.name, local_addr, e);
                     })
    };

    handle.spawn(msg);
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
