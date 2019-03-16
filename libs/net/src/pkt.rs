use bincode;
use failure::{bail, format_err, Error};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

// ---

/// `Packet` struct wraps a message in protocol-specific data.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum Packet<T: Clone + Debug + PartialEq> {
    Ack { ack: u32 },
    Reliable { seq: u32, msg: T },
    Unreliable { msg: T },
}

impl<T: Clone + Debug + PartialEq> Packet<T> {
    /// Encode this structure and give us a byte representation of it
    pub fn encode(&self) -> Result<Vec<u8>, Error>
    where
        T: Serialize,
    {
        let ser = bincode::serialize(self).map_err(|_| format_err!("Failed to serialize"))?;
        if ser.len() > Self::max_payload_size() {
            bail![
                "Payload exceeded size limit: {} > {}",
                ser.len(),
                Self::max_payload_size()
            ];
        }
        Ok(ser)
    }

    /// Decode a raw byte slice and return a `Packet`
    pub fn decode<'a>(data: &'a [u8]) -> Result<Packet<T>, Error>
    where
        T: Deserialize<'a>,
    {
        bincode::deserialize(&data[..]).map_err(|_| format_err!("Failed to deserialize"))
    }

    pub fn max_payload_size() -> usize {
        // const MIN_MTU_DATA_LINK_LAYER_SIZE: usize = 576;
        // const IP_HEADER_SIZE: usize = 20;
        // const UDP_HEADER_SIZE: usize = 8;
        // const MAXIMUM_PAYLOAD_SIZE: usize = MIN_MTU_DATA_LINK_LAYER_SIZE - IP_HEADER_SIZE - UDP_HEADER_SIZE;
        4 * 1000 * 1000
    }
}

// ---

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
