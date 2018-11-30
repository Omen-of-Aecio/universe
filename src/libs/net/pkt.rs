use glocals::Error;
use libs::net::msg::Message;

use bincode;

////////////
// Packet //
////////////

/// `Packet` struct wraps a message in protocol-specific data.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum Packet {
    Unreliable { msg: Message },
    Reliable { seq: u32, msg: Message },
    Ack { ack: u32 },
}
impl Packet {
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        let r = bincode::serialize(self).map_err(|_| format_err!("failed to serialize"))?;
        if r.len() as u32 > Packet::max_payload_size() {
            // TODO is it possible to somehow break the package up?
            Err(format_err!(
                "Tried to send too big Packet of size {}.",
                r.len()
            ))
        } else {
            Ok(r)
        }
    }
    pub fn decode(data: &[u8]) -> Result<Packet, Error> {
        bincode::deserialize(&data).map_err(|_| format_err!("failed to deserialize"))
    }
    pub fn max_payload_size() -> u32 {
        4 * 1024 // 4 KB
    }
}
