use failure::Error;
use failure::format_err;
use serde::{Deserialize, Serialize};
use serde_derive::{Deserialize, Serialize};
use std::fmt::Debug;

use bincode;

////////////
// Packet //
////////////

/// `Packet` struct wraps a message in protocol-specific data.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Packet<T: Clone + Debug + Eq + PartialEq> {
    Ack { ack: u32 },
    Reliable { seq: u32, msg: T },
    Unreliable { msg: T },
}

impl<'a, T: Clone + Debug + Deserialize<'a> + Serialize + Eq + PartialEq> Packet<T> {
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        let r = bincode::serialize(self).map_err(|_| format_err!("failed to serialize"))?;
        if r.len() as u32 > Packet::<T>::max_payload_size() {
            // TODO is it possible to somehow break the package up?
            Err(format_err!(
                "Tried to send too big Packet of size {}.",
                r.len()
            ))
        } else {
            Ok(r)
        }
    }

    pub fn decode(data: &'a [u8]) -> Result<Packet<T>, Error> {
        bincode::deserialize(&data[..]).map_err(|_| format_err!("failed to deserialize"))
    }

    pub fn max_payload_size() -> u32 {
        // 4 * 1024 // 4 KB
        4 * 1000 * 1000
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_test() {
        let message = Packet::Reliable { seq: 0, msg: false }.encode().unwrap();
        assert_eq![
            Packet::Reliable { seq: 0, msg: false },
            Packet::<bool>::decode(&message).unwrap()
        ];
    }

    #[test]
    fn fail_encoding_if_exceeding_or_equal_max_payload_size() {
        static OVERHEAD: usize = 16;

        let max = vec![0u8; Packet::<bool>::max_payload_size() as usize - OVERHEAD];
        assert![Packet::Reliable { seq: 0, msg: max }.encode().is_ok()];

        let max = vec![0u8; Packet::<bool>::max_payload_size() as usize];
        assert![Packet::Reliable { seq: 0, msg: max }.encode().is_err()];

        let max = vec![0u8; Packet::<bool>::max_payload_size() as usize + 1usize];
        assert![Packet::Reliable { seq: 0, msg: max }.encode().is_err()];
    }
}
