
use input::PlayerInput;
use geometry::vec::Vec2;
use world::color::Color;
#[derive(RustcEncodable, RustcDecodable, Clone)]
pub enum Message {
    // Messages from server
    Welcome {width: usize, height: usize, you_index: usize, players: Vec<Color>, white_base: Vec2, black_base: Vec2},
    WorldRect {x: usize, y: usize, width: usize, pixels: Vec<u8>},
    PlayerPos (Vec<Vec2>),
    NewPlayer {nr: u32, color: Color},


    // Messages from client
    Join,
    Input (PlayerInput),
    ToggleGravity,
    BulletFire { direction: Vec2 },
}

