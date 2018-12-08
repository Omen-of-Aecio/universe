use crate::glocals::Error;
use std::fmt::Debug;
use serde::{Deserialize, Serialize};
use serde_derive::{Deserialize, Serialize};
use failure::format_err;
use std::marker::PhantomData;

use bincode;

////////////
// Packet //
////////////

/// `Packet` struct wraps a message in protocol-specific data.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum Packet<'a, T: Clone + Debug + Deserialize<'a> + Serialize> {
    Ack { ack: u32, phantom: PhantomData<&'a T> },
    Reliable { seq: u32, msg: T },
    Unreliable { msg: T },
}

impl<'a, T: Clone + Debug + Deserialize<'a> + Serialize> Packet<'a, T> {
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

    pub fn decode(data: &[u8]) -> Result<Packet<'a, T>, Error> {
        bincode::deserialize(&data).map_err(|_| format_err!("failed to deserialize"))
    }

    pub fn max_payload_size() -> u32 {
        4 * 1024 // 4 KB
    }
}
