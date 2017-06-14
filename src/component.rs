use geometry::vec::Vec2;
use specs;
use specs::Component;

#[derive(Copy, Clone, Default, RustcEncodable, RustcDecodable, Debug)]
pub struct Pos {
    /// Position
    pub transl: Vec2,
    /// Orientation
    pub angular: f32,
}
impl Pos {
    pub fn with_transl(transl: Vec2) -> Pos {
        Pos {
            transl: transl,
            angular: 0.0
        }
    }
}


#[derive(Copy, Clone, Default, RustcEncodable, RustcDecodable, Debug)]
pub struct Vel {
    /// Positional velocity
    pub transl: Vec2,
    /// Angular speed
    pub angular: f32,
}


#[derive(Copy, Clone, Default, RustcEncodable, RustcDecodable, Debug)]
pub struct Force {
    /// Translational force
    pub transl: Vec2,
    /// Torque
    pub angular: f32,
}


#[derive(Copy, Clone, RustcEncodable, RustcDecodable, Debug)]
pub enum Jump {
    Active {
        // state
        progress: u32,
        // config
        /// Duration of jump in frames
        frames: u32,
        /// Force to apply every frame (for now just acceleration)
        force: f32,
    },
    Inactive,
}

impl Jump {
    pub fn new_active(frames: u32, force: f32) -> Jump {
        Jump::Active {
            progress: 0,
            frames: frames,
            force: force,
        }
    }
    pub fn is_active(&self) -> bool {
        match *self {
            Jump::Active {..} => true,
            Jump::Inactive => false,
        }
    }

    /// Returns acceleration upward for this frame
    /// Returns None if jump is done.
    pub fn tick(&mut self) -> Option<f32> {
        match self {
            &mut Jump::Active {mut progress, frames, force} => {
                progress += 1;
                if progress <= frames {
                    Some(force)
                } else {
                    None
                }
            }
            &mut Jump::Inactive => {
                None
            }
        }
    }

    pub fn get_progress(&self) -> Option<u32> {
        match *self {
            Jump::Active { progress, ..} => Some(progress),
            Jump::Inactive => None,
        }
    }
}


#[derive(Clone, RustcEncodable, RustcDecodable, Debug)]
pub struct Shape {
    pub points: Vec<(f32, f32)>, // Vec<Vec2> later. Now: for convenience with TileNet
}

impl Shape {
    pub fn new_quad(width: f32, height: f32) -> Shape {
        let mut points = Vec::new();
        points.push((0.0, 0.0));
        points.push((0.0, height));
        points.push((width, height));
        points.push((width, 0.0));
        Shape { points: points }
    }
}


#[derive(Copy, Clone, RustcEncodable, RustcDecodable, Debug)]
pub enum Color {
    White, Black,
}

impl Color {
    pub fn to_rgb(&self) -> [f32; 3] {
        match self {
            &Color::Black => [0.0, 0.0, 0.0],
            &Color::White => [1.0, 1.0, 1.0],
        }
    }
    pub fn to_intensity(&self) -> f32 {
        match self {
            &Color::Black => 0.0,
            &Color::White => 1.0,
        }
    }
}

/// Marks the object as a player
#[derive(Copy, Clone, Default, Debug)]
pub struct Player {
    pub id: u32
}
impl Player {
    pub fn new(id: u32) -> Player {
        Player {
            id: id
        }
    }
}


#[derive(Copy, Clone, Default, Debug, RustcEncodable, RustcDecodable)]
pub struct PlayerInput {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub g: bool,
}



//
// Specifying storage for the different components
//

impl Component for Pos {
    type Storage = specs::VecStorage<Pos>;
}
impl Component for Vel {
    type Storage = specs::VecStorage<Vel>;
}
impl Component for Force {
    type Storage = specs::VecStorage<Force>;
}
impl Component for Jump {
    type Storage = specs::VecStorage<Jump>;
}
impl Component for Shape {
    type Storage = specs::VecStorage<Shape>;
}
impl Component for Color {
    type Storage = specs::VecStorage<Color>;
}
impl Component for Player {
    type Storage = specs::VecStorage<Player>;
}
impl Component for PlayerInput {
    type Storage = specs::VecStorage<PlayerInput>;
}
