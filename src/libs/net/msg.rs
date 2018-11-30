//! Note on snapshots:
//! Snapshots are incremental: only that which has changed is sent to clients.
//! Only upon explicit request (or join) of a client does the client
//! receive a complete snapshot. This snapshot should be transmitted reliably.
use glocals::component::*;
use libs::geometry::vec::Vec2;
use addons::srv::diff::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Message {
    // Messages from server
    Welcome {
        width: u32,
        height: u32,
        you: u32,
        white_base: Vec2,
        black_base: Vec2,
    },
    WorldRect {
        x: usize,
        y: usize,
        width: usize,
        pixels: Vec<u8>,
    },
    State(Snapshot),

    // Messages from client
    Join {
        snapshot_rate: f32,
    },
    Input(PlayerInput),
    ToggleGravity,
    BulletFire {
        direction: Vec2,
    },
}
