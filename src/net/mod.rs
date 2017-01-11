pub mod msg;
mod conn;
mod pkt;

use net::msg::Message;
use net::conn::Connection;
use net::pkt::Packet;

use std;
use std::time::Duration;
use std::net::{UdpSocket, SocketAddr};
use std::iter::Iterator;
use std::collections::hash_map::HashMap;
use err::*;
use time::precise_time_ns;



/// Provides an interface to encode and send, decode and receive messages to/from any destination.
/// Reliability is provided through each Connection.

// Ideas

// Socket can be run in its own thread, where it keeps an event list (of future events) and counts
// down to the next event.
//
// Events are really just resending unacked packets.
//
// The event list may be just a list of the sent byte-arrays, together with the time in which they
// were sent and the sequence number.
//
// It has an interface to the outside world, on which you can queue packets and receive (iterate)
// packets. I think it will be adequate for now to use non-blocking socket for reception.

// - Potiential problem
// What if Client is waiting for a specific packet from Server - e.g the Welcome packet may come
// after some WorldPiece packets :o
//
// I think we need strictly in-order delivery of packets from Socket - at least until we find
// something better.


// ALTERNATIVE TO IN-ORDER DELIVERY
// - Client asks for a specific packet number if it needs to (e.g. it knows it will be the next
// packet)
// - Else, Client accepts whatever packet
// - Acknowledging happens by 






const RESEND_INTERVAL_MS: u64 = 1000;

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
            buffer: default_vec(Packet::max_packet_size()),
        })
    }

    /// A temporary simple (but brute force) solution to the need of occationally resending packets
    /// which haven't been acknowledged.
    pub fn update(&mut self) -> Result<()>{
        let socket = self.socket.try_clone()?; // because of aliasing
        for (addr, conn) in self.connections.iter_mut() {
            let to_resend = conn.get_resend_queue();
            debug!("To resend: {}", to_resend.len());
            for pkt in to_resend {
                socket.send_to(&pkt, *addr)?;
            }
            
        }

        Ok(())
    }


    /// Attempt to clone the socket to create consumable iterator
    pub fn messages(&mut self) -> SocketIter {
        SocketIter {
            socket: self,
        }
    }
    pub fn max_packet_size() -> usize {
        Packet::max_packet_size() // because Packet should probably be private to the net module
    }

    pub fn send_to(&mut self, msg: Message, dest: SocketAddr) -> Result<()> {
        let buffer = Packet::Unreliable {msg: msg}.encode();
        self.socket.send_to(&buffer, dest)?;

        Ok(())
    }

    pub fn send_reliably_to(&mut self, msg: Message, dest: SocketAddr) -> Result<()> {
        // Need to clone it here because of aliasing :/
        // IDEA: Could also let Connection::wrap_message take a clone of the UdpSocket and send the
        // message itself. Connection could even know the UdpSocket and SocketAddr
        let socket = self.socket.try_clone()?;

        let buffer = {
            let mut conn = self.get_connection_or_create(dest);
            debug!("Send reliably");
            conn.wrap_message(msg)
        };
        socket.send_to(&buffer, dest)?;
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
        self._recv()
    }

    fn _recv(&mut self) -> Result<(SocketAddr, Message)> {
        let socket = self.socket.try_clone()?;
        // Since we may just receive an Ack, we loop until we receive an actual message
        Ok(loop {
            let (amt, src) = self.socket.recv_from(&mut self.buffer)?;
            let packet = Packet::decode(&self.buffer[0..amt])?;
            let conn = self.get_connection_or_create(src);
            let (msg, reply) = conn.unwrap_message(packet)?;
            if let Some(msg) = msg {
                // Send Ack back if needed
                if let Some(reply) = reply {
                    let buffer = reply.encode();
                    debug!("send ack :O");
                    socket.send_to(&buffer, src);
                }
                break (src, msg);
            }
        })
    }
}

pub struct SocketIter<'a> {
    socket: &'a mut Socket,
}

impl<'a> Iterator for SocketIter<'a> {
    type Item = Result<(SocketAddr, Message)>;

    // TODO HERE the problem now is that _recv blocks.........
    fn next(&mut self) -> Option<Result<(SocketAddr, Message)>> {
        match self.socket.socket.set_nonblocking(true) {
            Err(e) => return Some(Err(e.into())),
            Ok(_) => {},
        }

        let msg = self.socket._recv();

        match msg {
            Ok(msg) => {
                Some(Ok(msg))
            },
            Err(e) => {
                let end_messages = match e {
                    Error(ErrorKind::Io(ref ioerr), _) => {
                        match ioerr.kind() {
                            std::io::ErrorKind::WouldBlock => true,
                            _ => false,
                        }
                    },
                    _ => false,
                };
                if end_messages {
                    None
                } else {
                    Some(Err(e))
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
