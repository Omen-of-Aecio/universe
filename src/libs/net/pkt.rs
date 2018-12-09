use crate::glocals::Error;
use failure::format_err;
use serde::{Deserialize, Serialize};
use serde_derive::{Deserialize, Serialize};
use std::fmt::Debug;

use bincode;

////////////
// Packet //
////////////

/// `Packet` struct wraps a message in protocol-specific data.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Packet<T: Clone + Debug> {
    Ack { ack: u32 },
    Reliable { seq: u32, msg: T },
    Unreliable { msg: T },
}

impl<'a, T: Clone + Debug + Deserialize<'a> + Serialize> Packet<T> {
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
        bincode::deserialize(&data).map_err(|_| format_err!("failed to deserialize"))
    }

    pub fn max_payload_size() -> u32 {
        4 * 1024 // 4 KB
    }
}
