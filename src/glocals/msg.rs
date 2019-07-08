use crate::game::Id;
use crate::game::PlayerData;
use bincode;
use failure::Error;

/// Message sent between from client to server
#[derive(Serialize, Deserialize)]
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
#[derive(Serialize, Deserialize)]
pub enum ServerMessage {
    Welcome { your_id: Id },
    State { players: Vec<PlayerData> },
}
impl ServerMessage {
    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }
    pub fn deserialize(bytes: &[u8]) -> Result<Self, Error> {
        Ok(bincode::deserialize(bytes)?)
    }
}

#[derive(Serialize, Deserialize, Copy, Clone)]
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
#[derive(Serialize, Deserialize, Copy, Clone)]
pub struct InputCommand {
    pub is_pressed: bool,
    pub key: InputKey,
}
