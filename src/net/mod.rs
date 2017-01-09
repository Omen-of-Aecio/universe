pub mod msg;
mod conn;
mod pkt;

use net::msg::Message;
use net::conn::Connection;
use net::pkt::{Packet, PacketKind};
use geometry::vec::Vec2;
use input::PlayerInput;

use std;
use std::mem::size_of_val;
use std::time::Duration;
use std::net::{UdpSocket, SocketAddr};
use std::iter::Iterator;
use std::mem::discriminant;
use std::io::Cursor;
use std::collections::hash_map::HashMap;
use err::{Result, Error};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use bincode::rustc_serialize::{encode, decode, DecodingError, DecodingResult};
use bincode;

use num_traits::int::PrimInt;


/// Provides an interface to encode and send, decode and receive messages to/from any destination.
/// Reliability is provided through each Connection.

const PROTOCOL: u32 = 0xf5ad9165;
const N: u32 = 10; // max packet size = 2^N


////////////
// Socket //
////////////

pub struct Socket {
    socket: UdpSocket,
    connections: HashMap<SocketAddr, Connection>,
    buffer: Vec<u8>,
}
impl Socket {
    pub fn new(port: u16) -> Result<Socket> {
        Ok(Socket {
            socket: UdpSocket::bind(("0.0.0.0:".to_string() + port.to_string().as_str()).as_str())?,
            connections: HashMap::new(),
            buffer: default_vec(2.pow(N)),
        })
    }

    pub fn max_packet_size() -> usize {
        2.pow(N) + 100
    }

    /// Attempt to clone the socket to create consumable iterator
    pub fn messages(&self) -> Result<SocketIter> {
        Ok(SocketIter {
            socket: self.socket.try_clone()?,
            buffer: default_vec(Socket::max_packet_size()),
        })
    }

    pub fn send_to(&self, msg: Message, dest: SocketAddr) -> Result<()> {
        let conn = self.get_connection_or_create(dest);
        let mut buffer = conn.wrap_unreliable_message(msg);
        self.socket.send_to(&buffer, dest)?;
        Ok(())
    }



    pub fn send_reliable_to(&self, msg: Message, dest: SocketAddr) -> Result<()> {
        let conn = self.get_connection_or_create(dest);
        let mut buffer = conn.wrap_message(msg);
        self.socket.send_to(buffer, dest)?;
        Ok(())
    }

    fn get_connection_or_create(&mut self, dest: SocketAddr) -> Connection{
        match self.connections.get(&dest) {
            None => {
                let conn = Connection::new();
                self.connections.insert(dest, conn);
                conn
            },
            Some(conn) => *conn,
            
        }
    }

    // Should wait until the packet is acknowledged..:
    // pub fn send_reliable_to_and_wait(&self, msg: Message, dest: SocketAddr) -> Result<()>

    /// Blocking, with timeout (3 sec)
    pub fn recv(&mut self) -> Result<(SocketAddr, Message)> {
        self.socket.set_nonblocking(false);
        self.socket.set_read_timeout(Some(Duration::new(3, 0)));

        let recv_result = self.socket.recv_from(&mut self.buffer);
        match recv_result {
            Ok((amt, src)) => {
                let conn = self.get_connection_or_create(src);
                let msg = conn.unwrap_message(&self.buffer[0..amt])?;
                Ok((src, msg))

            },
            Err(e) => Err(e.into()),
        }
    }
}

impl Iterator for Socket {
    type Item = Result<(SocketAddr, Message)>;

    fn next(&mut self) -> Option<Result<(SocketAddr, Message)>> {
        self.socket.set_nonblocking(true).unwrap();

        let msg = self.socket.recv_from(&mut self.buffer);
        match msg {
            Ok((amt, src)) => {
                let conn = self.get_connection_or_create(src);
                Some(conn.unwrap_message(&self.buffer[0..amt]).map(|msg| (src, msg)))
            },
            Err(e) => {
                if let std::io::ErrorKind::WouldBlock = e.kind() {
                    // There are no more messages
                    None
                } else {
                    // Some error Occured
                    Some(Err(e.into()))
                }
            },
        }
    }
}

fn default_vec<T: Default>(size: usize) -> Vec<T> {
    let mut zero_vec: Vec<T> = Vec::with_capacity(size);
    for i in 0..size {
        zero_vec.push(T::default());
    }
    return zero_vec;
}
