use std;
use std::mem::size_of_val;
use std::io::Cursor;
use std::time::Duration;
use std::net::{UdpSocket, SocketAddr};
use std::iter::Iterator;
use std::mem::discriminant;
use err::{Result, Error};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use bincode::rustc_serialize::{encode, decode, DecodingError, DecodingResult};
use bincode;

use num_traits::int::PrimInt;

#[derive(RustcEncodable, RustcDecodable)]
pub enum Message {
    // Messages from server
    WorldMeta {width: usize, height: usize},
    WorldRect {x: usize, y: usize, width: usize, height: usize, pixels: Vec<u8>},

    // Messages from client
    Join,
}

impl Message {
    fn encode(&self) -> Vec<u8> {
        encode(&self, bincode::SizeLimit::Bounded((Socket::max_packet_size()) as u64)).unwrap()
    }

    fn decode(data: &[u8]) -> Result<Message> {
        let mut rdr = Cursor::new(data);
        Socket::check_protocol(&mut rdr)?;

        let msg: DecodingResult<Message> = decode(&data[4..]);
        match msg {
            Ok(msg) => Ok(msg),
            Err(DecodingError::IoError(e)) => Err(e.into()),
            Err(e) => Err(e.into())
        }
    }
}


const PROTOCOL: u32 = 0xf5ad9165;
const N: u32 = 10; // max packet size = 2^N

pub struct Socket {
    socket: UdpSocket,
    buffer: Vec<u8>,
}
impl Socket {
    pub fn new(port: u16) -> Socket {
        let mut socket = Socket {
            socket: UdpSocket::bind(("0.0.0.0:".to_string() + port.to_string().as_str()).as_str()).unwrap(),
            buffer: default_vec(2.pow(N)),
        };
        socket
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

    pub fn send_to(&mut self, msg: Message, dest: SocketAddr) {
        let mut buffer = Vec::new();
        buffer.write_u32::<BigEndian>(PROTOCOL);
        buffer.extend(msg.encode());
        self.socket.send_to(&buffer, dest).unwrap();
    }

    /// Blocking, with timeout (3 sec)
    pub fn recv(&mut self) -> Result<(SocketAddr, Message)> {
        self.socket.set_nonblocking(false);
        self.socket.set_read_timeout(Some(Duration::new(3, 0)));
        let msg = self.socket.recv_from(&mut self.buffer);
        match msg {
            Ok((amt, src)) => Ok( (src, Message::decode(&self.buffer[0..amt])?) ),
            Err(e) => Err(e.into()),
        }
    }


    /// Returns number of bytes read if protocol number matches
    fn check_protocol(rdr: &mut Cursor<&[u8]>) -> Result<usize> {
        let msg_protocol = rdr.read_u32::<BigEndian>()?;
        if msg_protocol == PROTOCOL {
            Ok(size_of_val(&PROTOCOL))
        } else {
            Err(Error::WrongProtocol)
        }
    }

}

/// Just an iterator for nonblocking messages from UDP
pub struct SocketIter {
    socket: UdpSocket,
    buffer: Vec<u8>,
}

impl Iterator for SocketIter {
    type Item = Result<(SocketAddr, Message)>;

    fn next(&mut self) -> Option<Result<(SocketAddr, Message)>> {
        self.socket.set_nonblocking(true).unwrap();

        let msg = self.socket.recv_from(&mut self.buffer);
        match msg {
            Ok((amt, src)) => Some(Message::decode(&self.buffer[0..amt]).map(|msg| (src, msg))),
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
