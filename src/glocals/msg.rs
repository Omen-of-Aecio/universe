use crate::game::{Bullet, Id, PlayerData};
use bincode;
use failure::Error;

/// Message sent between from client to server
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClientMessage {
    Join,
    /// `mouse_pos` is the position of mouse in world space
    Input {
        commands: Vec<InputCommand>,
        mouse_pos: (f32, f32),
    },
}
impl ClientMessage {
    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }
    pub fn deserialize(bytes: &[u8]) -> Result<Self, Error> {
        Ok(bincode::deserialize(bytes)?)
    }
}

/// Message sent between from server to client
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerMessage {
    Welcome {
        your_id: Id,
        world_width: usize,
        world_height: usize,
        world_seed: [f32; 3],
    },
    /// Full state, sent unreliably.
    /// Could potentially also be used for a part of the state. In any case, the client
    /// is not supposed to e.g. delete a player or bullet that is not present in such state.
    /// Deletion of entities happens via `ServerMessage::DeltaState`
    State {
        players: Vec<PlayerData>,
        bullets: Vec<Bullet>,
    },
    /// Part of state update that is represented by a _change_, and thus sent _reliably_.
    DeltaState { removed: Vec<(Id, EntityType)> },
}
impl ServerMessage {
    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }
    pub fn deserialize(bytes: &[u8]) -> Result<Self, Error> {
        Ok(bincode::deserialize(bytes)?)
    }
}
#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub enum EntityType {
    Player,
    Bullet,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub enum InputKey {
    Up = 0,
    Down,
    Left,
    Right,
    LShift,
    LeftMouse, // IMPORTANT: LeftMouse has to be the very last (used to count variants)
}
/// Command that client sends to server that stems from user input.
/// Should all be sent reliably.
#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub struct InputCommand {
    pub is_pressed: bool,
    pub key: InputKey,
}
