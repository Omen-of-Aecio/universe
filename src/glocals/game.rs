use tilenet::TileNet;

use libs::geometry::cam::Camera;

use libs::geometry::vec::Vec2;
use libs::net::msg::Message;
use specs;
use specs::{Dispatcher, Join, LazyUpdate, World};
use std::cmp::min;

use std::collections::HashMap;
use std::vec::Vec;

pub struct Game {
    // pub world: World,
    pub cam: Camera,

    pub you: u32,

    pub white_base: Vec2,
    pub black_base: Vec2,

    // Extra graphics data (for debugging/visualization)
    pub vectors: Vec<(Vec2, Vec2)>,

    pub cam_mode: CameraMode,
}

/* Should go, together with some logic, to some camera module (?) */
#[derive(Copy, Clone)]
#[allow(unused)]
pub enum CameraMode {
    Interactive,
    FollowPlayer,
}
