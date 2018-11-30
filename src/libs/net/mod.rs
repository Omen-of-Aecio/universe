mod conn;
pub mod msg;
mod pkt;

use self::conn::Connection;
use libs::net::msg::Message;
use self::pkt::Packet;
use glocals::Error;

use std;
use std::collections::hash_map::HashMap;
use std::iter::Iterator;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::time::Duration;

/// Provides an interface to encode and [reliably] send, decode and receive messages to/from any destination.

// - maybe use concurrency for timing rather than polling

////////////
// Socket //
////////////

pub struct Socket {
    socket: UdpSocket,
    connections: HashMap<SocketAddr, Connection>,
    buffer: Vec<u8>,
}
impl Socket {
    pub fn new(port: u16) -> Result<Socket, Error> {
        Ok(Socket {
            socket: UdpSocket::bind(("0.0.0.0:".to_string() + port.to_string().as_str()).as_str())?,
            connections: HashMap::new(),
            buffer: default_vec(Packet::max_payload_size() as usize + 100),
            // XXX 100 as a safe bet (headers and such)
        })
    }

    /// A temporary (TODO) simple (but brute force) solution to the need of occasionally resending
    /// packets which haven't been acknowledged.
    pub fn update(&mut self) -> Result<(), Error> {
        let socket = self.socket.try_clone()?; // because of aliasing
        for (addr, conn) in self.connections.iter_mut() {
            let to_resend = conn.get_resend_queue();
            for pkt in to_resend {
                socket.send_to(&pkt, *addr)?;
            }
        }

        Ok(())
    }

    /// Attempt to clone the socket to create consumable iterator
    pub fn messages(&mut self) -> SocketIter {
        SocketIter { socket: self }
    }
    pub fn max_payload_size() -> u32 {
        Packet::max_payload_size() // because Packet should probably be private to the net module
    }

    /// Send unreliable message
    pub fn send_to(&mut self, msg: Message, dest: SocketAddr) -> Result<(), Error> {
        let buffer = Packet::Unreliable { msg }.encode()?;
        self.socket.send_to(&buffer, dest)?;

        Ok(())
    }

    /// Send reliable message
    pub fn send_reliably_to<'a>(
        &'a mut self,
        msg: Message,
        dest: SocketAddr,
        ack_handler: Option<Box<Fn() + 'static>>,
    ) -> Result<(), Error> {
        // Need to clone it here because of aliasing :/
        // IDEA: Could also let Connection::wrap_message take a clone of the UdpSocket and send the
        // message itself. Connection could even know the UdpSocket and SocketAddr
        let socket = self.socket.try_clone()?;

        let conn = self.get_connection_or_create(dest);
        conn.send_message(msg, &socket, ack_handler)?;
        Ok(())
    }

    fn get_connection_or_create(&mut self, dest: SocketAddr) -> &mut Connection {
        if self.connections.get(&dest).is_none() {
            let conn = Connection::new(dest);
            self.connections.insert(dest, conn);
        }
        self.connections.get_mut(&dest).unwrap()
    }

    // Should wait until the packet is acknowledged..:
    // pub fn send_reliable_to_and_wait(&self, msg: Message, dest: SocketAddr) -> Result<()>

    /// Blocking, with timeout (3 sec)
    pub fn recv(&mut self) -> Result<(SocketAddr, Message), Error> {
        self.socket.set_nonblocking(false)?;
        self.socket.set_read_timeout(Some(Duration::new(3, 0)))?;
        self._recv()
    }

    fn _recv(&mut self) -> Result<(SocketAddr, Message), Error> {
        // Since we may just receive an Ack, we loop until we receive an actual message
        Ok(loop {
            let socket = self.socket.try_clone()?;
            let (amt, src) = self.socket.recv_from(&mut self.buffer)?;
            let packet = Packet::decode(&self.buffer[0..amt])?;
            let conn = self.get_connection_or_create(src);
            let msg = conn.unwrap_message(packet, &socket)?;
            if let Some(msg) = msg {
                break (src, msg);
            }
        })
    }
}

pub struct SocketIter<'a> {
    socket: &'a mut Socket,
}

impl<'a> Iterator for SocketIter<'a> {
    type Item = Result<(SocketAddr, Message), Error>;

    fn next(&mut self) -> Option<Result<(SocketAddr, Message), Error>> {
        if let Err(e) = self.socket.socket.set_nonblocking(true) {
            return Some(Err(e.into()));
        }

        let msg = self.socket._recv();

        match msg {
            Ok(msg) => Some(Ok(msg)),
            Err(e) => {
                let end_messages = match e.downcast_ref::<std::io::Error>().map(|e| e.kind()) {
                    Some(std::io::ErrorKind::WouldBlock) => true,
                    _ => false,
                };
                if end_messages {
                    None
                } else {
                    Some(Err(e))
                }
            }
        }
    }
}

pub fn to_socket_addr(addr: &str) -> Result<SocketAddr, Error> {
    // Assume IPv4. Try to parse.
    let parts: Vec<&str> = addr.split(':').collect();
    if parts.len() != 2 {
        bail!("IP address must be on the form X.X.X.X:port");
    }

    let addr: Vec<u8> = parts[0]
        .split('.')
        .map(|x| x.parse::<u8>().unwrap())
        .collect();
    if addr.len() != 4 {
        bail!("IP address must be on the form X.X.X.X:port");
    }

    let port = parts[1].parse::<u16>().unwrap();

    Ok(SocketAddr::V4(SocketAddrV4::new(
        Ipv4Addr::new(addr[0], addr[1], addr[2], addr[3]),
        port,
    )))
}

fn default_vec<T: Default>(size: usize) -> Vec<T> {
    let mut zero_vec: Vec<T> = Vec::with_capacity(size);
    for _ in 0..size {
        zero_vec.push(T::default());
    }
    zero_vec
}
