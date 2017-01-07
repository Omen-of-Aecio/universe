
use input::PlayerInput;
use geometry::vec::Vec2;
use world::color::Color;
use err::Result;
use net::Socket;
use std::io::Cursor;
use bincode;
use bincode::rustc_serialize::{encode, decode, DecodingError, DecodingResult};
#[derive(RustcEncodable, RustcDecodable)]
pub enum Message {
    // Messages from server
    Welcome {width: usize, height: usize, you_index: usize, players: Vec<Color>, white_base: Vec2, black_base: Vec2},
    WorldRect {x: usize, y: usize, width: usize, height: usize, pixels: Vec<u8>},
    PlayerPos (Vec<Vec2>),
    

    // Messages from client
    Join,
    Input (PlayerInput),
    ToggleGravity,
}

impl Message {
    pub fn encode(&self) -> Vec<u8> {
        encode(&self, bincode::SizeLimit::Bounded((Socket::max_packet_size()) as u64)).unwrap()
    }

    pub fn decode(data: &[u8]) -> Result<Message> {
        let mut rdr = Cursor::new(data);
        Socket::check_protocol(&mut rdr)?;

        let msg: DecodingResult<Message> = decode(&data[4..]);
        match msg {
            Ok(msg) => Ok(msg),
            Err(DecodingError::IoError(e)) => Err(e.into()),
            Err(e) => Err(e.into())
        }
    }
}
