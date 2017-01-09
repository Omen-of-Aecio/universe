
use input::PlayerInput;
use geometry::vec::Vec2;
use world::color::Color;
use err::Result;
use net::Socket;
use std::io::Cursor;
use bincode;
use bincode::rustc_serialize::{encode, decode, DecodingError, DecodingResult};
#[derive(RustcEncodable, RustcDecodable, Clone)]
pub enum Message {
    None,

    // Messages from server
    Welcome {width: usize, height: usize, you_index: usize, players: Vec<Color>, white_base: Vec2, black_base: Vec2},
    WorldRect {x: usize, y: usize, width: usize, height: usize, pixels: Vec<u8>},
    PlayerPos (Vec<Vec2>),
    

    // Messages from client
    Join,
    Input (PlayerInput),
    ToggleGravity,
}

