use tilenet::TileNet;

use super::cam::Camera;

use component::*;
use geometry::vec::Vec2;
use glium;
use glium::glutin::VirtualKeyCode as KeyCode;
use global::Tile;
use input::Input;
use net::msg::Message;
use specs;
use specs::{Dispatcher, Join, LazyUpdate, World};
use srv::diff::{Entity, Snapshot};
use std::cmp::min;

use std::collections::HashMap;
use std::vec::Vec;

pub struct Game {
    pub world: World,
    pub cam: Camera,

    pub you: u32,

    pub white_base: Vec2,
    pub black_base: Vec2,

    // Extra graphics data (for debugging/visualization)
    pub vectors: Vec<(Vec2, Vec2)>,

    cam_mode: CameraMode,
}

impl Game {
    pub fn new(
        width: u32,
        height: u32,
        you: u32,
        white_base: Vec2,
        black_base: Vec2,
        display: &glium::Display,
    ) -> Game {
        let mut cam = Camera::default();
        cam.update_win_size(display);

        let world = {
            let mut w = World::new();
            // All components types should be registered before working with them
            w.register_with_storage::<_, Pos>(ComponentStorage::normal);
            w.register_with_storage::<_, Vel>(ComponentStorage::normal);
            w.register_with_storage::<_, Force>(ComponentStorage::normal);
            w.register_with_storage::<_, Jump>(ComponentStorage::normal);
            w.register_with_storage::<_, Shape>(ComponentStorage::normal);
            w.register_with_storage::<_, Color>(ComponentStorage::normal);
            w.register_with_storage::<_, Player>(ComponentStorage::normal);
            w.register_with_storage::<_, UniqueId>(ComponentStorage::normal);

            // The ECS system owns the TileNet
            let mut tilenet = TileNet::<Tile>::new(width as usize, height as usize);

            w.add_resource(tilenet);
            w.add_resource(cam);
            w.add_resource(HashMap::<u32, specs::Entity>::new());

            w
        };

        Game {
            world,
            cam,
            you,
            white_base,
            black_base,
            vectors: Vec::new(),
            cam_mode: CameraMode::FollowPlayer,
        }
    }

    /// Returns (messages to send, messages to send reliably)
    pub fn update(
        &mut self,
        dispatcher: &mut Dispatcher,
        input: &Input,
    ) -> (Vec<Message>, Vec<Message>) {
        self.world.maintain();
        // ^^ XXX maintain before rest, because previously in this frame we handled input & network pacakets
        self.vectors.clear(); // clear debug geometry
        let ret = self.handle_input(input);
        if let (CameraMode::FollowPlayer, Some(transl)) = (self.cam_mode, self.get_player_transl())
        {
            self.cam.center = transl;
        }
        *self.world.write_resource() = self.cam;
        dispatcher.dispatch(&self.world.res);
        ret
    }

    /// Returns (messages to send, messages to send reliably)
    fn handle_input(&mut self, input: &Input) -> (Vec<Message>, Vec<Message>) {
        let mut msg = Vec::new();
        let mut msg_reliable = Vec::new();
        if input.key_toggled_down(KeyCode::G) {
            msg.push(Message::ToggleGravity)
        }

        // Zooming..
        if input.key_down(KeyCode::N) {
            self.cam.zoom += 0.1;
        }
        if input.key_down(KeyCode::E) {
            self.cam.zoom -= 0.1;
        }

        // Mouse
        if input.mouse() {
            // Update camera offset //
            if let CameraMode::Interactive = self.cam_mode {
                let mut offset = input.mouse_moved() / self.cam.zoom;
                offset.x = -offset.x;
                self.cam.center += offset;
            }

            // Fire weapon //
            if let Some(transl) = self.get_player_transl() {
                let mouse_world_pos = self.cam.screen_to_world(input.mouse_pos());
                let dir = mouse_world_pos - transl;
                let msg = Message::BulletFire { direction: dir };
                msg_reliable.push(msg);
            }
        }

        // Zooming
        const ZOOM_FACTOR: f32 = 1.2;
        let y = input.mouse_wheel();
        if y > 0.0 {
            self.cam.zoom *= f32::powf(ZOOM_FACTOR, y as f32);
        } else if y < 0.0 {
            self.cam.zoom /= f32::powf(ZOOM_FACTOR, -y as f32);
        }

        msg_reliable.push(Message::Input(input.create_player_input()));
        (msg, msg_reliable)
    }

    /// Returns (white count, black count)
    pub fn count_player_colors(&self) -> (u32, u32) {
        let mut count = (0, 0);
        let (player, color) = {
            (
                self.world.read_storage::<Player>(),
                self.world.read_storage::<Color>(),
            )
        };
        for (_, color) in (&player, &color).join() {
            match *color {
                Color::Black => count.0 += 1,
                Color::White => count.1 += 1,
            }
        }
        count
    }

    // Access //
    pub fn get_tilenet_serial_rect(&self, x: usize, y: usize, w: usize, h: usize) -> Vec<Tile> {
        let tilenet = &*self.world.read_resource::<TileNet<Tile>>();
        let w = min(x + w, tilenet.get_size().0) as isize - x as isize;
        let h = min(y + h, tilenet.get_size().1) as isize - y as isize;
        if w == 0 || h == 0 {
            return Vec::new();
        }
        let w = w as usize;
        let h = h as usize;

        let pixels: Vec<u8> = tilenet
            .view_box((x, x + w, y, y + h))
            .map(|x| *x.0)
            .collect();
        assert!(pixels.len() == w * h);
        pixels
    }

    pub fn get_player_transl(&self) -> Option<Vec2> {
        let pos = self.world.read_storage::<Pos>();
        self.get_you()
            .and_then(|you| pos.get(you).map(|pos| pos.transl))
    }
    pub fn get_you(&self) -> Option<specs::Entity> {
        self.get_entity(self.you)
    }
    pub fn get_entity(&self, id: u32) -> Option<specs::Entity> {
        self.world
            .read_resource::<HashMap<u32, specs::Entity>>()
            .get(&id).cloned()
    }
    /// Puts entity mapping into the HashMap resource. The HashMap is maintained every frame so
    /// this only needs to be done when it otherwise poses a problem that the hashmap is not
    /// immediately updated.
    pub fn register_entity(&mut self, id: u32, ent: specs::Entity) {
        self.world
            .write_resource::<HashMap<u32, specs::Entity>>()
            .insert(id, ent);
    }
    pub fn apply_snapshot(&mut self, snapshot: Snapshot) {
        let mut added_entities: Vec<(u32, specs::Entity)> = Vec::new();
        {
            let updater = self.world.read_resource::<LazyUpdate>();
            for (id, entity) in snapshot.entities.into_iter() {
                match entity {
                    Some(Entity { components }) => {
                        match self.get_entity(id) {
                            Some(this_ent) => {
                                components.modify_existing(&*updater, this_ent);
                            }
                            None => {
                                // TODO: maybe need to care about type (Player/Bullet)
                                let ent = components.insert(&*updater, &*self.world.entities(), id);
                                added_entities.push((id, ent));
                            }
                        }
                    }
                    // This means the entity was deleted
                    None => match self.get_entity(id) {
                        Some(this_ent) => {
                            self.world.entities().delete(this_ent).unwrap();
                        }
                        None => error!("Server removed entity not owned by me"),
                    },
                }
            }
        }
        for (id, ent) in added_entities {
            self.register_entity(id, ent);
        }
    }

    pub fn print(&self) {
        // info!("TileNet"; "content" => format!["{:?}", self.get_tilenet()]);
    }
}

/* Should go, together with some logic, to some camera module (?) */
#[derive(Copy, Clone)]
#[allow(unused)]
enum CameraMode {
    Interactive,
    FollowPlayer,
}
