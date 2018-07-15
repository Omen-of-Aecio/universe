use net::msg::Message;
use err::*;


use bincode;
use bincode::rustc_serialize::{encode, decode, DecodingResult};
use num_traits::int::PrimInt;

const N: u32 = 10; // max packet size = 2^N

////////////
// Packet //
////////////

/// `Packet` struct wraps a message in protocol-specific data.
#[derive(Clone, RustcEncodable, RustcDecodable)]
pub enum Packet {
    Unreliable {
        msg: Message,
    },
    Reliable {
        seq: u32,
        msg: Message,
    },
    Ack {
        ack: u32,
    },

}
impl Packet{ 
    pub fn encode(&self) -> Vec<u8> {
        encode(self, bincode::SizeLimit::Bounded((Packet::max_packet_size()) as u64)).unwrap()
    }
    pub fn decode(data: &[u8]) -> Result<Packet> {
        let msg: DecodingResult<Packet> = decode(&data);
        msg.chain_err(|| "Error in decoding the data. Perhaps the received packet was too big?")
    }
    pub fn max_packet_size() -> u32 {
        2.pow(N) + 100
    }
}
