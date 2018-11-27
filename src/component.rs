use geometry::vec::Vec2;
use global::Tile;
use specs;
use specs::Component;
use tilenet::TileNet;

#[derive(Copy, Clone, Default, Serialize, Deserialize, Debug, Hash, Eq, PartialEq)]
pub struct UniqueId(pub u32);

#[derive(Copy, Clone, Default, Serialize, Deserialize, Debug)]
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
            angular: 0.0,
        }
    }
}

#[derive(Copy, Clone, Default, Serialize, Deserialize, Debug)]
pub struct Vel {
    /// Positional velocity
    pub transl: Vec2,
    /// Angular speed
    pub angular: f32,
}

#[derive(Copy, Clone, Default, Serialize, Deserialize, Debug)]
pub struct Force {
    /// Translational force
    pub transl: Vec2,
    /// Torque
    pub angular: f32,
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub enum Jump {
    Active {
        // state
        /// How many seconds have elapsed
        progress: f32,
        // config
        /// Duration of jump in frames (constant)
        duration: f32,
        /// Force to apply every frame (for now just acceleration) (constant)
        force: f32,
    },
    Inactive,
}

impl Jump {
    pub fn new_active(duration: f32, force: f32) -> Jump {
        Jump::Active {
            progress: 0.0,
            duration: duration,
            force: force,
        }
    }
    pub fn is_active(&self) -> bool {
        match *self {
            Jump::Active { .. } => true,
            Jump::Inactive => false,
        }
    }

    /// Returns acceleration upward for this frame
    /// Returns None if jump is done.
    pub fn tick(&mut self, delta_time: f32) -> Option<f32> {
        match *self {
            Jump::Active {
                ref mut progress,
                duration,
                force,
            } => {
                *progress += delta_time;
                if *progress <= duration {
                    Some(force * delta_time)
                } else {
                    None
                }
            }
            Jump::Inactive => None,
        }
    }

    pub fn get_progress(&self) -> Option<f32> {
        match *self {
            Jump::Active { progress, .. } => Some(progress),
            Jump::Inactive => None,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
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

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub enum Color {
    White,
    Black,
}

impl Default for Color {
    fn default() -> Color {
        Color::White
    }
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
pub struct Player;

#[derive(Copy, Clone, Default, Debug, Serialize, Deserialize)]
pub struct PlayerInput {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub g: bool,
}

use std::sync::{Arc, Mutex};
type Explosion = Fn((i32, i32), &Vel, &mut TileNet<Tile>) + Send;
// #[derive(Debug)]
pub struct Bullet {
    pub explosion: Arc<Mutex<Explosion>>,
}
impl Bullet {
    pub fn new<F: Fn((i32, i32), &Vel, &mut TileNet<Tile>) + Send + 'static>(
        explosion: F,
    ) -> Bullet {
        Bullet {
            explosion: Arc::new(Mutex::new(explosion)),
        }
    }
    pub fn explode(&self, pos: (i32, i32), vel: &Vel, t: &mut TileNet<Tile>) {
        self.explosion.lock().unwrap()(pos, vel, t)
    }
}

/// Marks deletion of an entity - to be deleted in the `DiffSys` system.
pub struct Delete;

//
// Specifying storage for the different components
//
use hibitset::BitSetLike;
use specs::{storage::UnprotectedStorage, world::Index};

pub enum ComponentStorage<C, S>
where
    C: Component,
    S: UnprotectedStorage<C> + Default,
{
    Normal(S),
    Flagged(specs::FlaggedStorage<C, S>),
}
impl<C, S> ComponentStorage<C, S>
where
    C: Component,
    S: UnprotectedStorage<C> + Default,
{
    pub fn normal() -> Self {
        ComponentStorage::Normal(S::default())
    }
    pub fn flagged() -> Self {
        ComponentStorage::Flagged(specs::FlaggedStorage::default())
    }
}

impl<C, S> Default for ComponentStorage<C, S>
where
    C: Component,
    S: UnprotectedStorage<C> + Default,
{
    fn default() -> Self {
        ComponentStorage::Normal(S::default())
    }
}

impl<C, S> UnprotectedStorage<C> for ComponentStorage<C, S>
where
    C: Component,
    S: UnprotectedStorage<C> + Default,
{
    unsafe fn clean<B>(&mut self, has: B)
    where
        B: BitSetLike,
    {
        match *self {
            ComponentStorage::Normal(ref mut storage) => storage.clean(has),
            ComponentStorage::Flagged(ref mut storage) => storage.clean(has),
        }
    }
    unsafe fn get(&self, id: Index) -> &C {
        match *self {
            ComponentStorage::Normal(ref storage) => storage.get(id),
            ComponentStorage::Flagged(ref storage) => storage.get(id),
        }
    }
    unsafe fn get_mut(&mut self, id: Index) -> &mut C {
        match *self {
            ComponentStorage::Normal(ref mut storage) => storage.get_mut(id),
            ComponentStorage::Flagged(ref mut storage) => storage.get_mut(id),
        }
    }
    unsafe fn insert(&mut self, id: Index, value: C) {
        match *self {
            ComponentStorage::Normal(ref mut storage) => storage.insert(id, value),
            ComponentStorage::Flagged(ref mut storage) => storage.insert(id, value),
        }
    }
    unsafe fn remove(&mut self, id: Index) -> C {
        match *self {
            ComponentStorage::Normal(ref mut storage) => storage.remove(id),
            ComponentStorage::Flagged(ref mut storage) => storage.remove(id),
        }
    }
}

impl<C, S> specs::Tracked for ComponentStorage<C, S>
where
    C: Component,
    S: UnprotectedStorage<C> + Default,
{
    fn channels(&self) -> &specs::storage::TrackChannels {
        match *self {
            ComponentStorage::Normal(ref storage) => {
                panic!("Using `ComponentStorage::Normal` as Tracked!")
            }
            ComponentStorage::Flagged(ref storage) => storage.channels(),
        }
    }
    fn channels_mut(&mut self) -> &mut specs::storage::TrackChannels {
        match *self {
            ComponentStorage::Normal(ref mut storage) => {
                panic!("Using `ComponentStorage::Normal` as Tracked!")
            }
            ComponentStorage::Flagged(ref mut storage) => storage.channels_mut(),
        }
    }
}

impl Component for UniqueId {
    type Storage = ComponentStorage<Self, specs::VecStorage<Self>>;
}
impl Component for Pos {
    type Storage = ComponentStorage<Self, specs::VecStorage<Self>>;
}
impl Component for Vel {
    type Storage = ComponentStorage<Self, specs::VecStorage<Self>>;
}
impl Component for Force {
    type Storage = ComponentStorage<Self, specs::VecStorage<Self>>;
}
impl Component for Jump {
    type Storage = ComponentStorage<Self, specs::VecStorage<Self>>;
}
impl Component for Shape {
    type Storage = ComponentStorage<Self, specs::VecStorage<Self>>;
}
impl Component for Color {
    type Storage = ComponentStorage<Self, specs::VecStorage<Self>>;
}
impl Component for Player {
    type Storage = ComponentStorage<Self, specs::VecStorage<Self>>;
}
impl Component for Bullet {
    type Storage = ComponentStorage<Self, specs::VecStorage<Self>>;
}
impl Component for PlayerInput {
    type Storage = ComponentStorage<Self, specs::VecStorage<Self>>;
}
impl Component for Delete {
    type Storage = ComponentStorage<Self, specs::VecStorage<Self>>; // TODO - NullStorage?
}
