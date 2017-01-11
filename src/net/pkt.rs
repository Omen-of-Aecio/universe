use net::msg::Message;
use net::Socket;
use err::*;


use bincode;
use bincode::rustc_serialize::{encode, decode, DecodingError, DecodingResult};
use num_traits::int::PrimInt;

const PROTOCOL: u32 = 0xf5ad9165;
const N: u32 = 10; // max packet size = 2^N

////////////
// Packet //
////////////

#[derive(Clone, RustcEncodable, RustcDecodable)]
pub enum PacketKind {
    Unreliable {ack: u32},
    Reliable {ack: u32, seq: u32},
}

/// `Packet` struct wraps a message in protocol-specific data.
#[derive(Clone, RustcEncodable, RustcDecodable)]
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
    pub fn check_protocol_nr(&self) -> Result<()> {
        match self.protocol_nr {
            PROTOCOL => Ok(()),
            _ => Err(ErrorKind::WrongProtocol.into()),
        }
    }
    pub fn encode(&self) -> Vec<u8> {
        encode(self, bincode::SizeLimit::Bounded((Packet::max_packet_size()) as u64)).unwrap()
    }
    pub fn decode(data: &[u8]) -> Result<Packet> {
        let msg: DecodingResult<Packet> = decode(&data);
        msg.chain_err(|| "Error in decoding the data. Perhaps the received packet was too big?")
    }
    pub fn max_packet_size() -> usize {
        2.pow(N) + 100
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
