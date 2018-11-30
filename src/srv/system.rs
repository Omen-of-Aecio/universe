use collision::{bullet_move, player_move};
use component::*;
use geometry::Vec2;
use global::Tile;
use glocals::GameConfig;
use specs::{self, prelude::*};
use srv::diff::*;
use std::collections::HashMap;
use super::DeltaTime;
use tilenet::TileNet;

////////////
// Server

// (TODO extra friction when on ground?)

pub struct JumpSys;
impl<'a> specs::System<'a> for JumpSys {
    type SystemData = (
        Read<'a, DeltaTime>,
        Read<'a, GameConfig>,
        WriteStorage<'a, Jump>,
        WriteStorage<'a, Vel>,
    );
    fn run(&mut self, data: Self::SystemData) {
        let (delta_time, config, mut jump, mut vel) = data;
        for (jump, vel) in (&mut jump, &mut vel).join() {
            // Jump
            let acc = jump.tick(delta_time.secs);
            let progress = jump.get_progress();
            if let Some(acc) = acc {
                vel.transl.y += acc;
            }
            if let Some(progress) = progress {
                if progress > config.jump_delay {
                    *jump = Jump::Inactive; // Regain jumping (like a sort of double jump)
                }
            }
        }
    }
}

type MoveSysSpec<'a> = (
    Read<'a, DeltaTime>,
    Read<'a, LazyUpdate>,
    Read<'a, GameConfig>,
    WriteExpect<'a, TileNet<Tile>>,
    Entities<'a>,
    ReadStorage<'a, Player>,
    ReadStorage<'a, Bullet>,
    WriteStorage<'a, Pos>,
    WriteStorage<'a, Vel>,
    ReadStorage<'a, Shape>,
    ReadStorage<'a, Color>,
);

/// For Player <-> TileNet collision
/// (TODO) general movement of objects that have Shape, Force, Vel, Pos, Color
pub struct MoveSys;
impl<'a> specs::System<'a> for MoveSys {
    type SystemData = MoveSysSpec<'a>;

    fn run(&mut self, data: Self::SystemData) {
        let (
            delta_time,
            updater,
            game_conf,
            mut tilenet,
            entities,
            player,
            bullet,
            mut pos,
            mut vel,
            shape,
            color,
        ) = data;
        let gravity = if game_conf.gravity_on {
            game_conf.gravity
        } else {
            Vec2::null_vec()
        };

        // Players
        for (_, pos, vel, shape, color) in (&player, &mut pos, &mut vel, &shape, &color).join() {
            let has_collided = player_move(pos, vel, shape, *color, &tilenet, delta_time.secs);
            // Friction
            vel.transl *= game_conf.air_fri; // TODO delta_time
            if has_collided {
                vel.transl.x *= game_conf.ground_fri;
            }
            // Gravity
            vel.transl += gravity * delta_time.secs;
        }
        // Bullets
        for (bullet, entity, pos, vel, shape, color) in
            (&bullet, &*entities, &mut pos, &mut vel, &shape, &color).join()
        {
            let (poc, has_collided) =
                bullet_move(pos, vel, shape, *color, &tilenet, delta_time.secs);
            // Friction
            vel.transl *= game_conf.air_fri;
            // Gravity
            vel.transl += gravity * delta_time.secs;
            // Effect
            if has_collided {
                bullet.explode((poc.0 as i32, poc.1 as i32), vel, &mut tilenet);
                updater.insert(entity, Delete);
                debug!("Bullet remove");
            }
        }
    }
}

type InputSysSpec<'a> = (
    Read<'a, DeltaTime>,
    Read<'a, GameConfig>,
    ReadStorage<'a, PlayerInput>,
    WriteStorage<'a, Jump>,
    WriteStorage<'a, Vel>,
);

pub struct InputSys;
impl<'a> specs::System<'a> for InputSys {
    type SystemData = InputSysSpec<'a>;

    fn run(&mut self, data: Self::SystemData) {
        let (delta_time, conf, input, mut jump, mut vel) = data;
        let t = delta_time.secs;
        for (input, jump, vel) in (&input, &mut jump, &mut vel).join() {
            if input.left {
                vel.transl.x -= conf.hori_acc * t;
            }
            if input.right {
                vel.transl.x += conf.hori_acc * t;
            }
            if input.up {
                if conf.gravity_on {
                    if !jump.is_active() {
                        vel.transl.y = 0.0;
                        *jump = Jump::new_active(conf.jump_duration, conf.jump_acc);
                    }
                } else {
                    vel.transl.y += conf.hori_acc * t;
                }
            }
            if input.down && !conf.gravity_on {
                vel.transl.y -= conf.hori_acc * t;
            }
        }
    }
}

/// System to create a HashMap<u32, Entity>, which must be done every frame. Used on both
/// client and server
pub struct MaintainSys;
impl<'a> specs::System<'a> for MaintainSys {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, UniqueId>,
        Write<'a, HashMap<u32, Entity>>,
    );
    fn run(&mut self, (entities, ids, mut map): Self::SystemData) {
        let mut new_map = HashMap::new();
        for (entity, id) in (&*entities, &ids).join() {
            new_map.insert(id.0, entity);
        }
        *map = new_map;
    }
}

type SimplerType<'a> = (
    WriteExpect<'a, DiffHistory>,
    Entities<'a>,
    ReadStorage<'a, UniqueId>,
    ReadStorage<'a, Delete>,
    ReadStorage<'a, Pos>,
    ReadStorage<'a, Shape>,
    ReadStorage<'a, Color>,
);
/// System to generate diffs (bitsets for inserted/modified and removed components).
/// Also deletes elements marked by deletion.
/// Used on server.
pub struct DiffSys;
impl<'a> specs::System<'a> for DiffSys {
    type SystemData = SimplerType<'a>;
    fn run(&mut self, (mut diffs, entities, id, delete, pos, shape, color): Self::SystemData) {
        // Delete entities marked for deletion
        let mut removed: Vec<UniqueId> = Vec::new();
        for (entity, id, _delete) in (&*entities, &id, &delete).join() {
            let _ignore_result = entities.delete(entity);
            removed.push(*id);
        }
        //
        diffs.add_diff(removed, &pos, &shape, &color);
    }
}
