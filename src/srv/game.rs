use tilenet::TileNet;

use err::*;

use std::cmp::min;
use net::msg::Message;
use global::Tile;
use net::msg::SrvPlayer;
use geometry::vec::Vec2;
use component::*;
use tilenet_gen;
use specs;
use specs::{Dispatcher, World, Join, Builder};

use std::collections::HashMap;
use std::vec::Vec;

use conf::Config;

pub struct Game {
    pub world: World,
    pub game_conf: GameConfig,

    players: HashMap<u32, specs::Entity>,
    player_id_seq: u32,

    /// Width of the generated world
    width: usize,
    /// Height of the generated world
    height: usize,


    pub white_base: Vec2,
    pub black_base: Vec2,

    // Extra graphics data (for debugging/visualization)
    pub vectors: Vec<(Vec2, Vec2)>,
}



impl Game {
    pub fn new(conf: Config, white_base: Vec2, black_base: Vec2) -> Game {
        let gc = GameConfig::new(&conf);

        let world = {
            let mut w = World::new();
            // All components types should be registered before working with them
            w.register::<Player>();
            w.register::<Bullet>();
            w.register::<Pos>();
            w.register::<Vel>();
            w.register::<Force>();
            w.register::<Shape>();
            w.register::<Color>();
            w.register::<Jump>();
            w.register::<PlayerInput>();
            
            // The ECS system owns the TileNet
            let mut tilenet = TileNet::<Tile>::new(conf.world.width as usize, conf.world.height as usize);


            // Create bases
            let base_size: usize = 24;
            let pos = (white_base.x as usize, white_base.y as usize);
            tilenet.set_box(&0, (pos.0 - base_size, pos.1 - base_size), (pos.0 + base_size, pos.1 + base_size));
            let pos = (black_base.x as usize, black_base.y as usize);
            tilenet.set_box(&255, (pos.0 - base_size, pos.1 - base_size), (pos.0 + base_size, pos.1 + base_size));
            
            w.add_resource(tilenet);
            w.add_resource(gc);
            w.add_resource(conf.clone());

            w
        };

        Game {
            world: world,
            game_conf: gc,
            players: HashMap::default(),
            player_id_seq: 0,
            width: conf.world.width as usize,
            height: conf.world.height as usize,
            white_base: white_base,
            black_base: black_base,
            vectors: Vec::new(),
        }
    }

    pub fn generate_world(&mut self) {
        tilenet_gen::proc1(&mut *self.world.write_resource::<TileNet<Tile>>());
        // world::gen::rings(&mut world.tilenet, 2);
    }


    /// Returns (messages to send, messages to send reliably)
    pub fn update(&mut self, dispatcher: &mut Dispatcher) -> (Vec<Message>, Vec<Message>) {
        self.vectors.clear(); // clear debug geometry
        *self.world.write_resource::<GameConfig>() = self.game_conf;
        dispatcher.dispatch(&mut self.world.res);
        self.world.maintain();
        (Vec::new(), Vec::new())
    }


    /// Returns (white count, black count)
    pub fn count_player_colors(&self) -> (u32, u32) {
        let mut count = (0, 0);
        let (player, color) = (self.world.read_storage::<Player>(), self.world.read_storage::<Color>());
        for (_, color) in (&player, &color).join() {
            match *color {
                Color::Black => count.0 += 1,
                Color::White => count.1 += 1,
            }
        }
        count
    }

    // Access //
    /// Return tilenet data as well as new cropped (w, h)
    pub fn get_tilenet_serial_rect(&self, x: usize, y: usize, w: usize, h: usize) -> (Vec<Tile>, usize, usize) {
        let tilenet = &*self.world.read_resource::<TileNet<Tile>>();
        let w = min(x + w, tilenet.get_size().0) as isize - x as isize;
        let h = min(y + h, tilenet.get_size().1) as isize - y as isize;
        if w <= 0 || h <= 0 {
            return (Vec::new(), 0, 0);
        }
        let w = w as usize;
        let h = h as usize;

        let pixels: Vec<u8> = tilenet.view_box((x, x+w, y, y+h)).map(|x| *x.0).collect();
        assert!(pixels.len() == w*h);
        (pixels, w, h)
    }
    pub fn get_player(&self, id: u32) -> specs::Entity {
        self.players[&id]
    }
    pub fn toggle_gravity(&mut self) {
        self.game_conf.gravity_on = !self.game_conf.gravity_on;
    }
    pub fn get_width(&self) -> usize {
        self.width
    }
    pub fn get_height(&self) -> usize {
        self.height
    }
    
    /// Create SrvPlayer from a player with id u32
    pub fn get_srv_player(&self, id: u32) -> Result<SrvPlayer> {
        let entity = *self.players.get(&id).ok_or_else(|| format!("Player with id {} not found.", id))?;
        let (player, color, pos) = (self.world.read_storage::<Player>(),
                                    self.world.read_storage::<Color>(),
                                    self.world.read_storage::<Pos>());
        let player = player.get(entity).ok_or_else(|| format!("Player with id {} not found.", id))?;
        let color = color.get(entity).ok_or_else(|| format!("Player with id {} not found.", id))?;
        let pos = pos.get(entity).ok_or_else(|| format!("Player with id {} not found.", id))?;
        Ok(SrvPlayer::new(player, *color, pos))
    }
    pub fn get_srv_players(&self) -> Vec<SrvPlayer> {
        let (player, color, pos) = (self.world.read_storage::<Player>(),
                                    self.world.read_storage::<Color>(),
                                    self.world.read_storage::<Pos>());
        (&player, &color, &pos).join().map(|data| {
            let (player, color, pos) = data;
            SrvPlayer::new(player, *color, pos)
        }).collect()
    }

    /// Add player if not already added
    pub fn add_player(&mut self, col: Color) -> u32 {
        self.player_id_seq += 1;
        let transl = match col {
            Color::White => Vec2::new(self.white_base.x, self.white_base.y),
            Color::Black => Vec2::new(self.black_base.x, self.black_base.y),
        };

        let entity = self.world.create_entity()
            .with(Player::new(self.player_id_seq))
            .with(Pos::with_transl(transl))
            .with(Vel::default())
            .with(Force::default())
            .with(Shape::new_quad(10.0, 10.0))
            .with(col)
            .with(Jump::Inactive)
            .with(PlayerInput::default())
            .build();
        self.players.insert(self.player_id_seq, entity);
        self.player_id_seq
    }

    pub fn bullet_fire(&mut self, player_id: u32, direction: Vec2) -> Result<()> {
        let entity = self.get_player(player_id);
        let (pos, color) = {
            let pos = self.world.read_storage::<Pos>();
            let col = self.world.read_storage::<Color>();
            (pos.get(entity).unwrap().clone(), col.get(entity).unwrap().clone())
        };
        let color2 = color.clone();
        let explosion = move |pos: (i32, i32), vel: &Vel, tilenet: &mut TileNet<Tile>| {
                tilenet.set(&((255.0 - color2.to_intensity()*255.0) as u8), (pos.0 as usize, pos.1 as usize));
            };
        let entity = self.world.create_entity()
            .with(Bullet::new(explosion))
            .with(pos)
            .with(Vel {transl: direction, angular: 1.0})
            .with(Force::default())
            .with(Shape::new_quad(4.0, 4.0))
            .with(color)
            .build();
        Ok(())
    }

    pub fn print(&self) {
        // info!("TileNet"; "content" => format!["{:?}", self.get_tilenet()]);
    }
}

pub fn map_tile_value_via_color(tile: &Tile, color: Color) -> Tile {
	match (tile, color) {
		(&0, Color::Black) => 255,
		(&255, Color::Black) => 0,
		_ => *tile,
	}
}


/* Should go, together with some logic, to some camera module (?) */

#[derive(Copy,Clone,Default)]
pub struct GameConfig {
    pub hori_acc: f32, 
    pub jump_duration: f32,
    pub jump_delay: f32,
    pub jump_acc: f32,
    pub gravity: Vec2,
    pub gravity_on: bool,
    pub srv_tick_duration: f32,
    pub air_fri: Vec2,
    pub ground_fri: f32,
}
impl GameConfig {
    pub fn new(conf: &Config) -> GameConfig {
        // Helper functions to convert seconds->ticks
        let conv_duration = |n: f32| {
            n * conf.srv.tps as f32
        };
        let conv_acc = |n: f32| {
            n / (conf.srv.tps * conf.srv.tps) as f32
        };
        GameConfig {
            hori_acc: conv_acc(conf.player.hori_acc),
            jump_duration: conf.player.jump_duration,
            jump_delay: conf.player.jump_delay,
            jump_acc: conv_acc(conf.player.jump_acc),
            gravity: Vec2::new(0.0, - conv_acc(conf.world.gravity)),
            gravity_on: false,
            srv_tick_duration: conf.get_srv_tick_duration(),
            air_fri: Vec2::new(conf.world.air_fri.0, conf.world.air_fri.1),
            ground_fri: conf.world.ground_fri,
        }
    }

}
