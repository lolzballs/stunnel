use config::Config;
use tunnel;

use std::collections::HashMap;
use std::ops::FnOnce;
use std::net::{SocketAddr, ToSocketAddrs};

use futures::{Async, Future, Poll, Stream};
use futures::stream::{Fuse, Map};
use tokio_core::net::{Incoming, TcpListener, TcpStream};
use tokio_core::reactor::{Core, Handle};

#[derive(Clone, Debug)]
struct Tunnel {
    name: String,
    remote: SocketAddr,
    sni_addr: String,
}

pub struct Server {
    tunnels: HashMap<SocketAddr, Tunnel>,
}

impl Server {
    pub fn from_config(config: Config) -> Self {
        let mut tunnels = HashMap::with_capacity(config.tunnels.len());
        for (name, tunnel) in config.tunnels {
            let local = tunnel
                .listen
                .to_socket_addrs()
                .unwrap()
                .next()
                .unwrap();
            let remote = tunnel
                .remote
                .to_socket_addrs()
                .unwrap()
                .next()
                .unwrap();
            let sni_addr = match tunnel.sni_addr {
                Some(ref s) => s.clone(),
                None => tunnel.remote.split(':').nth(0).unwrap().into(),
            };
            tunnels.insert(local,
                           Tunnel {
                               name: name,
                               remote: remote,
                               sni_addr: sni_addr,
                           });
        }

        Server { tunnels: tunnels }
    }

    pub fn start(self) {
        let mut core = Core::new().expect("could not create reactor core");
        let handle = core.handle();

        let mut all = Vec::new();
        for (local_addr, tunnel) in self.tunnels.iter() {
            let tunnel = tunnel.clone();
            let listen =
                Self::listen(&handle, local_addr.clone()).map(move |(sock, addr)| {
                                                                  (sock, addr, tunnel.clone())
                                                              });
            all.push(listen.fuse());
        }

        let all_incoming =
            select_all(all).for_each(|(sock, addr, tunnel)| {
                                         let local_addr = sock.local_addr().unwrap();
                                         Self::handle_client(&handle, &tunnel, sock, local_addr);
                                         Ok(())
                                     });
        core.run(all_incoming).unwrap();
    }

    fn listen(handle: &Handle, local_addr: SocketAddr) -> Incoming {
        let listener = TcpListener::bind(&local_addr, &handle).unwrap();
        listener.incoming()
    }

    fn handle_client(handle: &Handle,
                     tunnel: &Tunnel,
                     local_sock: TcpStream,
                     local_addr: SocketAddr) {
        println!("[{}]: recieved connection from {}", tunnel.name, local_addr);
        let remote_sock = TcpStream::connect(&tunnel.remote, &handle);

        tunnel::start_tunnel(&handle, local_sock, remote_sock, tunnel.sni_addr.clone());
    }
}

fn select_all<S: Stream>(streams: Vec<Fuse<S>>) -> Select<S> {
    Select {
        streams: streams,
        current: 0,
    }
}

struct Select<S> {
    streams: Vec<Fuse<S>>,
    current: usize,
}

impl<S: Stream> Stream for Select<S> {
    type Item = S::Item;
    type Error = S::Error;

    fn poll(&mut self) -> Poll<Option<S::Item>, S::Error> {
        let start = self.current;
        for stream in self.streams[start..].iter_mut() {
            let done = match try!(stream.poll()) {
                Async::Ready(Some(item)) => return Ok(Some(item).into()),
                Async::Ready(None) => true,
                Async::NotReady => false,
            };
            self.current += 1;
        }
        for stream in self.streams[..start].iter_mut() {
            let done = match try!(stream.poll()) {
                Async::Ready(Some(item)) => return Ok(Some(item).into()),
                Async::Ready(None) => true,
                Async::NotReady => false,
            };
            self.current += 1;
        }

        let stream_count = self.streams.len();
        if self.current > stream_count {
            self.current %= stream_count;
        }

        Ok(Async::NotReady)
    }
}