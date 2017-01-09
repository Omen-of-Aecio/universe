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
    pub fn messages(&mut self) -> SocketIter {
        SocketIter {
            socket: self,
        }
    }

    pub fn send_to(&mut self, msg: Message, dest: SocketAddr) -> Result<()> {
        let mut buffer = {
            let conn = self.get_connection_or_create(dest);
            conn.wrap_unreliable_message(msg)
        };
        self.socket.send_to(&buffer, dest)?;
        Ok(())
    }



    pub fn send_reliable_to(&mut self, msg: Message, dest: SocketAddr) -> Result<()> {
        // Need to clone it here because of aliasing :/
        // IDEA: Could also let Connection::wrap_message take a clone of the UdpSocket and send the
        // message itself. Connection could even know what UdpSocket it is associated with..
        let socket = self.socket.try_clone()?;

        let mut buffer = {
            let mut conn = self.get_connection_or_create(dest);
            conn.wrap_message(msg)
        };
        socket.send_to(buffer, dest)?;
        Ok(())
    }

    fn get_connection_or_create<'a>(&'a mut self, dest: SocketAddr) -> &'a mut Connection {
        match self.connections.get(&dest).is_some() {
            false => {
                let conn = Connection::new();
                self.connections.insert(dest, conn);
            },
            true => {},
        }
        self.connections.get_mut(&dest).unwrap()
    }

    // Should wait until the packet is acknowledged..:
    // pub fn send_reliable_to_and_wait(&self, msg: Message, dest: SocketAddr) -> Result<()>

    /// Blocking, with timeout (3 sec)
    pub fn recv(&mut self) -> Result<(SocketAddr, Message)> {
        self.socket.set_nonblocking(false)?;
        self.socket.set_read_timeout(Some(Duration::new(3, 0)))?;

        let recv_result = self.socket.recv_from(&mut self.buffer);
        match recv_result {
            Ok((amt, src)) => {
                let packet = Packet::decode(&self.buffer[0..amt])?;
                let conn = self.get_connection_or_create(src);
                let msg = conn.unwrap_message(packet)?;
                Ok((src, msg))

            },
            Err(e) => Err(e.into()),
        }
    }
}

pub struct SocketIter<'a> {
    socket: &'a mut Socket,
}

impl<'a> Iterator for SocketIter<'a> {
    type Item = Result<(SocketAddr, Message)>;

    fn next(&mut self) -> Option<Result<(SocketAddr, Message)>> {
        match self.socket.socket.set_nonblocking(true) {
            Err(e) => return Some(Err(e.into())),
            Ok(_) => {},
        }

        let msg = self.socket.socket.recv_from(&mut self.socket.buffer);
        match msg {
            Ok((amt, src)) => {
                let packet = match Packet::decode(&self.socket.buffer[0..amt]) {
                    Ok(p) => p,
                    Err(e) => return Some(Err(e)),
                };
                let conn = self.socket.get_connection_or_create(src);
                Some(conn.unwrap_message(packet).map(|msg| (src, msg)))
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
