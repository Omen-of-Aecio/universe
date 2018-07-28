use tilenet::TileNet;

use err::*;

use std::cmp::min;
use net::msg::Message;
use global::Tile;
use geometry::vec::Vec2;
use component::*;
use srv::diff::{DiffHistory, Snapshot};
use tilenet_gen;
use specs;
use specs::{Dispatcher, World, Join, Builder};

use std::collections::{BTreeMap, HashMap};
use std::vec::Vec;
use std::time::Duration;
use net::msg;

use conf::Config;

pub struct Game {
    frame: u32,
    pub world: World,
    pub game_conf: GameConfig,

    /// Mapping from unique ID to specs Entity
    entities: HashMap<u32, specs::Entity>,
    entity_id_seq: u32,

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
            w.register_with_storage::<_, Player>(|| ComponentStorage::normal());
            w.register_with_storage::<_, Bullet>(|| ComponentStorage::normal());
            w.register_with_storage::<_, Pos>(|| ComponentStorage::flagged());
            w.register_with_storage::<_, Vel>(|| ComponentStorage::normal());
            w.register_with_storage::<_, Force>(|| ComponentStorage::normal());
            w.register_with_storage::<_, Shape>(|| ComponentStorage::flagged());
            w.register_with_storage::<_, Color>(|| ComponentStorage::flagged());
            w.register_with_storage::<_, Jump>(|| ComponentStorage::normal());
            w.register_with_storage::<_, PlayerInput>(|| ComponentStorage::normal());
            w.register_with_storage::<_, UniqueId>(|| ComponentStorage::normal());
            
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
            w.add_resource(::DeltaTime::default());
            w.add_resource(HashMap::<u32, specs::Entity>::new());
            let dh = DiffHistory::new(&w); // (NLL)
            w.add_resource(dh);

            w
        };

        Game {
            frame: 0,
            world: world,
            game_conf: gc,
            entities: HashMap::default(),
            entity_id_seq: 0,
            width: conf.world.width as usize,
            height: conf.world.height as usize,
            white_base: white_base,
            black_base: black_base,
            vectors: Vec::new(),
        }
    }

    pub fn generate_world(&mut self) {
        let mut tilenet = self.world.write_resource::<TileNet<Tile>>();
        tilenet_gen::proc1(&mut *tilenet);

        // Create bases
        let base_size: usize = 24;
        let pos = (self.white_base.x as usize, self.white_base.y as usize);
        tilenet.set_box(&0, (pos.0 - base_size, pos.1 - base_size), (pos.0 + base_size, pos.1 + base_size));
        let pos = (self.black_base.x as usize, self.black_base.y as usize);
        tilenet.set_box(&255, (pos.0 - base_size, pos.1 - base_size), (pos.0 + base_size, pos.1 + base_size));
        // world::gen::rings(&mut world.tilenet, 2);
    }


    /// Returns (messages to send, messages to send reliably)
    pub fn update(&mut self, dispatcher: &mut Dispatcher, delta_time: ::DeltaTime) -> (Vec<Message>, Vec<Message>) {
        self.frame += 1;
        self.world.maintain();
        self.vectors.clear(); // clear debug geometry
        *self.world.write_resource::<GameConfig>() = self.game_conf;
        *self.world.write_resource::<::DeltaTime>() = delta_time;
        dispatcher.dispatch(&mut self.world.res);

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
    /// Return tilenet data as well as new cropped (w, h) to fit inside the world
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
    pub fn get_entity(&self, id: u32) -> specs::Entity {
        self.entities[&id]
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
    
    /// Add player if not already added. Return its unique ID
    pub fn add_player(&mut self, col: Color) -> u32 {
        self.entity_id_seq += 1;
        let transl = match col {
            Color::White => Vec2::new(self.white_base.x, self.white_base.y),
            Color::Black => Vec2::new(self.black_base.x, self.black_base.y),
        };

        info!("Add player"; "id" => self.entity_id_seq);
        let entity = self.world.create_entity()
            .with(UniqueId (self.entity_id_seq))
            .with(Player)
            .with(Pos::with_transl(transl))
            .with(Vel::default())
            .with(Force::default())
            .with(Shape::new_quad(10.0, 10.0))
            .with(col)
            .with(Jump::Inactive)
            .with(PlayerInput::default())
            .build();
        self.entities.insert(self.entity_id_seq, entity);
        self.entity_id_seq
    }

    pub fn bullet_fire(&mut self, player_id: u32, direction: Vec2) -> Result<(), Error> {
        let entity = self.get_entity(player_id);
        let (pos, color) = {
            let pos = self.world.read_storage::<Pos>();
            let col = self.world.read_storage::<Color>();
            (pos.get(entity).unwrap().clone(), col.get(entity).unwrap().clone())
        };
        let color2 = color.clone();
        let explosion = move |pos: (i32, i32), _vel: &Vel, tilenet: &mut TileNet<Tile>| {
                tilenet.set(&((255.0 - color2.to_intensity()*255.0) as u8), (pos.0 as usize, pos.1 as usize));
            };
        self.entity_id_seq += 1;
        let _entity = self.world.create_entity()
            .with(UniqueId (self.entity_id_seq))
            .with(Bullet::new(explosion))
            .with(pos)
            .with(Vel {transl: direction, angular: 1.0})
            .with(Force::default())
            .with(Shape::new_quad(4.0, 4.0))
            .with(color)
            .build();
        Ok(())
    }

    pub fn create_snapshot(&self, since_frame: u32) -> Snapshot {
        let diffs = self.world.read_resource::<DiffHistory>();
        diffs.create_snapshot(since_frame, self.frame, &self.world)
    }
    pub fn frame_nr(&self) -> u32 {
        self.frame
    }

    pub fn input(&mut self, id: u32, input: PlayerInput) -> Result<(), Error> {
        let entity = self.entities.get(&id).ok_or_else(|| format_err!("Entity not found"))?;
        let mut input_resource = self.world.write_storage::<PlayerInput>();
        let input_ref = input_resource.get_mut(*entity).ok_or_else(|| format_err!("Entity doesn't have input"))?;
        *input_ref = input;
        Ok(())
    }
}

pub fn map_tile_value_via_color(tile: &Tile, color: Color) -> Tile {
	match (tile, color) {
		(&0, Color::Black) => 255,
		(&255, Color::Black) => 0,
		_ => *tile,
	}
}


#[derive(Copy,Clone,Default)]
pub struct GameConfig {
    pub hori_acc: f32, 
    pub jump_duration: f32,
    pub jump_delay: f32,
    pub jump_acc: f32,
    pub gravity: Vec2,
    pub gravity_on: bool,
    pub srv_tick_duration: Duration,
    pub air_fri: Vec2,
    pub ground_fri: f32,
}
impl GameConfig {
    pub fn new(conf: &Config) -> GameConfig {
        GameConfig {
            hori_acc: conf.player.hori_acc,
            jump_duration: conf.player.jump_duration,
            jump_delay: conf.player.jump_delay,
            jump_acc: conf.player.jump_acc,
            gravity: Vec2::new(0.0, - conf.world.gravity),
            gravity_on: false,
            srv_tick_duration: conf.get_srv_tick_duration(),
            air_fri: Vec2::new(conf.world.air_fri.0, conf.world.air_fri.1),
            ground_fri: conf.world.ground_fri,
        }
    }

}
