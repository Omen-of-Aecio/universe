use net::msg::Message;
use err::*;


use num_traits::int::PrimInt;
use bincode;

const N: u32 = 10; // max packet size = 2^N

////////////
// Packet //
////////////

/// `Packet` struct wraps a message in protocol-specific data.
#[derive(Clone, Serialize, Deserialize)]
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
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        // TODO max packet size Packet::max_packet_size()
        bincode::serialize(self).map_err(|_| format_err!("failed to serialize"))
    }
    pub fn decode(data: &[u8]) -> Result<Packet, Error> {
        bincode::deserialize(&data).map_err(|_| format_err!("failed to deserialize"))
    }
    pub fn max_packet_size() -> u32 {
        2.pow(N) + 100
    }
}
