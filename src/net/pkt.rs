use net::msg::Message;
use net::Socket;
use err::{Error, Result};


use bincode;
use bincode::rustc_serialize::{encode, decode, DecodingError, DecodingResult};

const PROTOCOL: u32 = 0xf5ad9165;

////////////
// Packet //
////////////

#[derive(RustcEncodable, RustcDecodable)]
pub enum PacketKind {
    Unreliable {ack: u32},
    Reliable {ack: u32, seq: u32},
}

/// `Packet` struct wraps a message in protocol-specific data.
#[derive(RustcEncodable, RustcDecodable)]
pub struct Packet {
    protocol_nr: u32,
    pub kind: PacketKind,
    pub msg: Message,
}
impl Packet{ 
    pub fn new(kind: PacketKind, msg: Message) -> Packet {
        Packet {
            protocol_nr: PROTOCOL,
            kind: kind,
            msg: msg,
        }
    }
    pub fn encode(&self) -> Vec<u8> {
        encode(self, bincode::SizeLimit::Bounded((Socket::max_packet_size()) as u64)).unwrap()
    }
    pub fn decode(data: &[u8]) -> Result<Packet> {
        let msg: DecodingResult<Packet> = decode(&data);
        match msg {
            Ok(msg) => Ok(msg),
            Err(DecodingError::IoError(e)) => Err(e.into()),
            Err(e) => Err(e.into())
        }
    }
    pub fn check_protocol_nr(&self) -> Result<()> {
        match self.protocol_nr {
            PROTOCOL => Ok(()),
            _ => Err(Error::WrongProtocol),
        }
    }
}


/////////////////
// PacketState //
/////////////////

/// The state of a package sent reliably
enum PacketState {
    Waiting,
    Acknowledged,
}
