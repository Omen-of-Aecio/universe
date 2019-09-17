use crate::game::{Bullet, Id, PlayerData};
use bincode;
use failure::Error;
use std::convert::TryFrom;

/// Message sent between from client to server
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClientMessage {
    Join,
    Input(Vec<InputCommand>),
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
    DeltaState {
        removed: Vec<(Id, EntityType)>,
        grid_changes: Vec<(u32, u32, u8)>,
    },
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

// ---

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum InputCommand {
    Keyboard {
        state: winit::ElementState,
        virtual_keycode: winit::VirtualKeyCode,
        modifiers: winit::ModifiersState,
    },
    Mouse {
        position: (f32, f32),
        state: winit::ElementState,
        button: winit::MouseButton,
        modifiers: winit::ModifiersState,
    },
}

impl TryFrom<winit::KeyboardInput> for InputCommand {
    type Error = ();

    fn try_from(key: winit::KeyboardInput) -> Result<Self, Self::Error> {
        match key {
            winit::KeyboardInput {
                state,
                virtual_keycode: Some(virtual_keycode),
                modifiers,
                ..
            } => Ok(Self::Keyboard {
                state,
                virtual_keycode: virtual_keycode,
                modifiers,
            }),
            _ => Err(()),
        }
    }
}
