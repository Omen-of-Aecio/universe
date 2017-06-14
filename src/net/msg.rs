use geometry::vec::Vec2;
use component::*;

#[derive(RustcEncodable, RustcDecodable, Clone)]
pub struct SrvPlayer {
    pub id: u32,
    pub col: Color,
    pub pos: Vec2,
}
impl SrvPlayer {
    pub fn new(player: &Player, col: Color, pos: &Pos) -> SrvPlayer {
        SrvPlayer {
            id: player.id,
            col: col,
            pos: pos.transl,
        }
    }
}

#[derive(RustcEncodable, RustcDecodable, Clone)]
pub enum Message {
    // Messages from server
    Welcome {width: u32, height: u32, you: u32, players: Vec<SrvPlayer>, white_base: Vec2, black_base: Vec2},
    WorldRect {x: usize, y: usize, width: usize, pixels: Vec<u8>},
    Players (Vec<SrvPlayer>),
    NewPlayer (SrvPlayer),


    // Messages from client
    Join,
    Input (PlayerInput),
    ToggleGravity,
    BulletFire { direction: Vec2 },
}

