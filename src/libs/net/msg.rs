//! Note on snapshots:
//! Snapshots are incremental: only that which has changed is sent to clients.
//! Only upon explicit request (or join) of a client does the client
//! receive a complete snapshot. This snapshot should be transmitted reliably.

use serde_derive::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum Message {
    Packet(u32),
}
