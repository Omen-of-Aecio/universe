use super::diff::{DiffHistory, Snapshot};
use super::DeltaTime;
use addons::tilenet_gen;
use glocals::component::*;
use glocals::conf::Config;
use glocals::Tile;
use glocals::*;
use libs::geometry::vec::Vec2;
use libs::net::msg::Message;
use specs;
use specs::{Builder, Dispatcher, Join, World};
use std::cmp::min;
use std::collections::HashMap;
use std::vec::Vec;
use tilenet::TileNet;

pub fn create_servergame(conf: &Config, white_base: Vec2, black_base: Vec2) -> ServerGame {
    let gc = GameConfig::new(&conf);

    let world = {
        let mut w = World::new();
        // All components types should be registered before working with them
        w.register_with_storage::<_, Player>(ComponentStorage::normal);
        w.register_with_storage::<_, Bullet>(ComponentStorage::normal);
        w.register_with_storage::<_, Pos>(ComponentStorage::flagged);
        w.register_with_storage::<_, Vel>(ComponentStorage::normal);
        w.register_with_storage::<_, Force>(ComponentStorage::normal);
        w.register_with_storage::<_, Shape>(ComponentStorage::flagged);
        w.register_with_storage::<_, Color>(ComponentStorage::flagged);
        w.register_with_storage::<_, Jump>(ComponentStorage::normal);
        w.register_with_storage::<_, PlayerInput>(ComponentStorage::normal);
        w.register_with_storage::<_, UniqueId>(ComponentStorage::normal);
        w.register_with_storage::<_, Delete>(ComponentStorage::normal);

        // The ECS system owns the TileNet
        let mut tilenet =
            TileNet::<Tile>::new(conf.world.width as usize, conf.world.height as usize);

        // Create bases
        let base_size: usize = 24;
        let pos = (white_base.x as usize, white_base.y as usize);
        tilenet.set_box(
            &0,
            (pos.0 - base_size, pos.1 - base_size),
            (pos.0 + base_size, pos.1 + base_size),
        );
        let pos = (black_base.x as usize, black_base.y as usize);
        tilenet.set_box(
            &255,
            (pos.0 - base_size, pos.1 - base_size),
            (pos.0 + base_size, pos.1 + base_size),
        );

        w.add_resource(tilenet);
        w.add_resource(gc);
        w.add_resource(conf.clone());
        w.add_resource(DeltaTime::default());
        w.add_resource(HashMap::<u32, specs::Entity>::new());
        let dh = DiffHistory::new(&w); // (NLL)
        w.add_resource(dh);

        w
    };

    ServerGame {
        frame: 0,
        game_conf: gc,
        entities: HashMap::default(),
        entity_id_seq: 0,
        width: conf.world.width as usize,
        height: conf.world.height as usize,
        white_base,
        black_base,
        vectors: Vec::new(),
    }
}

/// Returns (messages to send, messages to send reliably)
pub fn update(
    s: &mut ServerGame,
    dispatcher: &mut Dispatcher,
    delta_time: DeltaTime,
) -> (Vec<Message>, Vec<Message>) {
    s.frame += 1;
    // s.world.maintain();
    s.vectors.clear(); // clear debug geometry
    // *s.world.write_resource::<GameConfig>() = s.game_conf;
    // *s.world.write_resource::<DeltaTime>() = delta_time;
    // dispatcher.dispatch(&s.world.res);

    (Vec::new(), Vec::new())
}

/// Returns (white count, black count)
pub fn count_player_colors(s: &ServerGame) -> (u32, u32) {
    // let mut count = (0, 0);
    // let (player, color) = (
    //     s.world.read_storage::<Player>(),
    //     s.world.read_storage::<Color>(),
    // );
    // for (_, color) in (&player, &color).join() {
    //     match *color {
    //         Color::Black => count.0 += 1,
    //         Color::White => count.1 += 1,
    //     }
    // }
    // count
    (0, 0)
}

// Access //
/// Return tilenet data as well as new cropped (w, h) to fit inside the world
pub fn get_tilenet_serial_rect(
    s: &ServerGame,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
) -> (Vec<Tile>, usize, usize) {
    // let tilenet = &*s.world.read_resource::<TileNet<Tile>>();
    // let w = min(x + w, tilenet.get_size().0) as isize - x as isize;
    // let h = min(y + h, tilenet.get_size().1) as isize - y as isize;
    // if w <= 0 || h <= 0 {
    //     return (Vec::new(), 0, 0);
    // }
    // let w = w as usize;
    // let h = h as usize;

    // let pixels: Vec<u8> = tilenet
    //     .view_box((x, x + w, y, y + h))
    //     .map(|x| *x.0)
    //     .collect();
    // assert!(pixels.len() == w * h);
    // (pixels, w, h)
    (Vec::new(), 0, 0)
}
pub fn get_entity(s: &ServerGame, id: u32) -> specs::Entity {
    s.entities[&id]
}
pub fn toggle_gravity(s: &mut ServerGame) {
    s.game_conf.gravity_on = !s.game_conf.gravity_on;
}
pub fn get_width(s: &ServerGame) -> usize {
    s.width
}
pub fn get_height(s: &ServerGame) -> usize {
    s.height
}

/// Add player if not already added. Return its unique ID
pub fn add_player(s: &mut ServerGame, col: Color) -> u32 {
    s.entity_id_seq += 1;
    let transl = match col {
        Color::White => Vec2::new(s.white_base.x, s.white_base.y),
        Color::Black => Vec2::new(s.black_base.x, s.black_base.y),
    };

    info!("Add player"; "id" => s.entity_id_seq);
    // let entity = s
    //     .world
    //     .create_entity()
    //     .with(UniqueId(s.entity_id_seq))
    //     .with(Player)
    //     .with(Pos::with_transl(transl))
    //     .with(Vel::default())
    //     .with(Force::default())
    //     .with(Shape::new_quad(10.0, 10.0))
    //     .with(col)
    //     .with(Jump::Inactive)
    //     .with(PlayerInput::default())
    //     .build();
    // s.entities.insert(s.entity_id_seq, entity);
    s.entity_id_seq
}

pub fn bullet_fire(s: &mut ServerGame, player_id: u32, direction: Vec2) -> Result<(), Error> {
    // let entity = get_entity(s, player_id);
    // let (pos, color) = {
    //     let pos = s.world.read_storage::<Pos>();
    //     let col = s.world.read_storage::<Color>();
    //     (*pos.get(entity).unwrap(), *col.get(entity).unwrap())
    // };
    // let color2 = color;
    // let explosion = move |pos: (i32, i32), _vel: &Vel, tilenet: &mut TileNet<Tile>| {
    //     tilenet.set(
    //         &((255.0 - color2.to_intensity() * 255.0) as u8),
    //         (pos.0 as usize, pos.1 as usize),
    //     );
    // };
    // s.entity_id_seq += 1;
    // let _entity = s
    //     .world
    //     .create_entity()
    //     .with(UniqueId(s.entity_id_seq))
    //     .with(Bullet::new(explosion))
    //     .with(pos)
    //     .with(Vel {
    //         transl: direction,
    //         angular: 1.0,
    //     })
    //     .with(Force::default())
    //     .with(Shape::new_quad(4.0, 4.0))
    //     .with(color)
    //     .build();
    Ok(())
}

// pub fn create_snapshot(s: &ServerGame, since_frame: u32) -> Snapshot {
    // let diffs = s.world.read_resource::<DiffHistory>();
    // diffs.create_snapshot(since_frame, s.frame, &s.world)
// }
pub fn frame_nr(s: &ServerGame) -> u32 {
    s.frame
}

pub fn input(s: &mut ServerGame, id: u32, input: PlayerInput) -> Result<(), Error> {
    // let entity = s
    //     .entities
    //     .get(&id)
    //     .ok_or_else(|| format_err!("Entity not found"))?;
    // let mut input_resource = s.world.write_storage::<PlayerInput>();
    // let input_ref = input_resource
    //     .get_mut(*entity)
    //     .ok_or_else(|| format_err!("Entity doesn't have input"))?;
    // *input_ref = input;
    Ok(())
}

pub fn map_tile_value_via_color(tile: Tile, color: Color) -> Tile {
    match (tile, color) {
        (0, Color::Black) => 255,
        (255, Color::Black) => 0,
        _ => tile,
    }
}

impl GameConfig {
    pub fn new(conf: &Config) -> GameConfig {
        GameConfig {
            hori_acc: conf.player.hori_acc,
            jump_duration: conf.player.jump_duration,
            jump_delay: conf.player.jump_delay,
            jump_acc: conf.player.jump_acc,
            gravity: Vec2::new(0.0, -conf.world.gravity),
            gravity_on: false,
            srv_tick_duration: conf.get_srv_tick_duration(),
            air_fri: Vec2::new(conf.world.air_fri.0, conf.world.air_fri.1),
            ground_fri: conf.world.ground_fri,
        }
    }
}

pub fn generate_world(s: &mut ServerGame) {
    // let mut tilenet = s.world.write_resource::<TileNet<Tile>>();
    // tilenet_gen::proc1(&mut *tilenet);

    // // Create bases
    // let base_size: usize = 24;
    // let pos = (s.white_base.x as usize, s.white_base.y as usize);
    // tilenet.set_box(
    //     &0,
    //     (pos.0 - base_size, pos.1 - base_size),
    //     (pos.0 + base_size, pos.1 + base_size),
    // );
    // let pos = (s.black_base.x as usize, s.black_base.y as usize);
    // tilenet.set_box(
    //     &255,
    //     (pos.0 - base_size, pos.1 - base_size),
    //     (pos.0 + base_size, pos.1 + base_size),
    // );
    // world::gen::rings(&mut world.tilenet, 2);
}
