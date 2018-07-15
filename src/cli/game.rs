use tilenet::TileNet;

use cli::cam::Camera;

use std::cmp::min;
use glium;
use glium::glutin::{VirtualKeyCode as KeyCode};
use net::msg::Message;
use input::Input;
use global::Tile;
use net::msg::SrvPlayer;
use geometry::vec::Vec2;
use component::*;
use specs;
use specs::{World, Join};

use std::collections::HashMap;
use std::vec::Vec;


pub struct Game {
    pub world: World,
    pub cam: Camera,

    players: HashMap<u32, specs::Entity>,
    you: u32,

    pub white_base: Vec2,
    pub black_base: Vec2,

    // Extra graphics data (for debugging/visualization)
    pub vectors: Vec<(Vec2, Vec2)>,

    cam_mode: CameraMode,
}



impl Game {
    pub fn new(width: u32, height: u32, you: u32, white_base: Vec2, black_base: Vec2, display: glium::Display) -> Game {
        let mut cam = Camera::new();
        cam.update_win_size(&display);

        let world = {
            let mut w = World::new();
            // All components types should be registered before working with them
            w.register::<Pos>();
            w.register::<Vel>();
            w.register::<Force>();
            w.register::<Jump>();
            w.register::<Shape>();
            w.register::<Color>();
            w.register::<Player>();
            
            // The ECS system owns the TileNet
            let mut tilenet = TileNet::<Tile>::new(width as usize, height as usize);


            // Create bases
            let base_size: usize = 24;
            let pos = (white_base.x as usize, white_base.y as usize);
            tilenet.set_box(&0, (pos.0 - base_size, pos.1 - base_size), (pos.0 + base_size, pos.1 + base_size));
            let pos = (black_base.x as usize, black_base.y as usize);
            tilenet.set_box(&255, (pos.0 - base_size, pos.1 - base_size), (pos.0 + base_size, pos.1 + base_size));
            
            w.add_resource(tilenet);
            w.add_resource(cam);

            w
        };


        Game {
            world: world,
            cam: cam,
            players: HashMap::default(),
            you: you,
            white_base: white_base,
            black_base: black_base,
            vectors: Vec::new(),
            cam_mode: CameraMode::FollowPlayer,
        }
    }

    /// Returns (messages to send, messages to send reliably)
    pub fn update(&mut self, input: &Input) -> (Vec<Message>, Vec<Message>) {
        self.vectors.clear(); // clear debug geometry
        let ret = self.handle_input(input);
        if let CameraMode::FollowPlayer = self.cam_mode {
            self.cam.center = self.get_player_transl();
        }
        *self.world.write_resource() = self.cam;
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
            let mouse_world_pos = self.cam.screen_to_world(input.mouse_pos());
            let dir = mouse_world_pos - self.get_player_transl();
            let msg = Message::BulletFire {direction: dir};
            msg_reliable.push(msg);
        }

        // Zooming
        const ZOOM_FACTOR: f32 = 1.2;
        let y = input.mouse_wheel();
        if y > 0.0 {
            self.cam.zoom *= f32::powf(ZOOM_FACTOR, y as f32);
        } else if y < 0.0 {
            self.cam.zoom /= f32::powf(ZOOM_FACTOR, -y as f32);
        }


        msg_reliable.push( Message::Input (input.create_player_input()) );
        (msg, msg_reliable)
    }


    /// Returns (white count, black count)
    pub fn count_player_colors(&self) -> (u32, u32) {
        let mut count = (0, 0);
        let (player, color) = {
            (self.world.read::<Player>(), self.world.read::<Color>())
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

        let pixels: Vec<u8> = tilenet.view_box((x, x+w, y, y+h)).map(|x| *x.0).collect();
        assert!(pixels.len() == w*h);
        pixels
    }

    pub fn get_player_transl(&self) -> Vec2 {
        let pos = self.world.read::<Pos>();
        pos.get(self.get_you()).unwrap().transl
    }
    pub fn get_you(&self) -> specs::Entity {
        *self.players.get(&self.you).unwrap()
    }
    pub fn get_player(&self, id: u32) -> specs::Entity {
        self.players[&id]
    }

    /// Add player if not already added
    pub fn update_player(&mut self, srv_player: SrvPlayer) {
        let mapping = self.players.get(&srv_player.id).map(|x| *x);
        match mapping {
            Some(ref entity) => {
                let (mut player, mut pos, mut color) = (self.world.write::<Player>(),
                                                        self.world.write::<Pos>(),
                                                        self.world.write::<Color>());
                player.get_mut(*entity).unwrap().id = srv_player.id;
                pos.get_mut(*entity).unwrap().transl = srv_player.pos;
                *color.get_mut(*entity).unwrap() = srv_player.col;


            },
            None => {
                let entity = self.world.create_entity()
                    .with(Player::new(srv_player.id))
                    .with(Pos::with_transl(srv_player.pos))
                    .with(Vel::default())
                    .with(Force::default())
                    .with(Shape::new_quad(10.0, 10.0))
                    .with(srv_player.col)
                    .with(Jump::Inactive)
                    .build();
                self.players.insert(srv_player.id, entity);
            }
        }
    }

    pub fn print(&self) {
        // info!("TileNet"; "content" => format!["{:?}", self.get_tilenet()]);
    }
}



/* Should go, together with some logic, to some camera module (?) */
#[derive(Copy,Clone)]
#[allow(unused)]
enum CameraMode {
    Interactive,
    FollowPlayer,
}
