use specs::{self, Read, WriteExpect, ReadStorage, WriteStorage, Join, Entities};
use collision::{player_move, bullet_move};
use component::*;
use tilenet::TileNet;
use global::Tile;
use srv::game::GameConfig;
use geometry::Vec2;
use ::DeltaTime;

////////////
// Server

pub struct JumpSys;
/// For Player <-> TileNet collision
/// (TODO) general movement of objects that have Shape, Force, Vel, Pos, Color
pub struct MoveSys;
pub struct InputSys;


// (TODO extra friction when on ground?)

impl<'a> specs::System<'a> for JumpSys {
    type SystemData = (Read<'a, DeltaTime>,
                       Read<'a, GameConfig>,
                       WriteStorage<'a, Jump>,
                       WriteStorage<'a, Vel>);
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

impl<'a> specs::System<'a> for MoveSys {
    type SystemData = (Read<'a, DeltaTime>,
                       Entities<'a>,
                       WriteExpect<'a, TileNet<Tile>>,
                       Read<'a, GameConfig>,
                       ReadStorage<'a, Player>,
                       ReadStorage<'a, Bullet>,
                       WriteStorage<'a, Pos>,
                       WriteStorage<'a, Vel>,
                       ReadStorage<'a, Shape>,
                       ReadStorage<'a, Color>);

    fn run(&mut self, data: Self::SystemData) {
        let (delta_time, entities, mut tilenet, game_conf, player, bullet, mut pos, mut vel, shape, color) = data;
        let gravity = if game_conf.gravity_on { game_conf.gravity } else { Vec2::null_vec() };

        // Players
        for (_, entity, pos, vel, shape, color) in
                (&player, &*entities, &mut pos, &mut vel, &shape, &color).join() {
            let has_collided = player_move(pos, vel, shape, color, &tilenet, delta_time.secs);
            // Friction
            vel.transl = vel.transl * game_conf.air_fri; // TODO delta_time
            if has_collided {
                vel.transl.x *= game_conf.ground_fri;
            }
            // Gravity
            vel.transl += gravity * delta_time.secs;
        }
        // Bullets
        for (bullet, entity, pos, vel, shape, color) in
                (&bullet, &*entities, &mut pos, &mut vel, &shape, &color).join() {
            let (poc, has_collided) = bullet_move(pos, vel, shape, color, &tilenet, delta_time.secs);
            // Friction
            vel.transl = vel.transl * game_conf.air_fri;
            // Gravity
            vel.transl += gravity * delta_time.secs;
            // Effect
            if has_collided {
                bullet.explode((poc.0 as i32, poc.1 as i32), vel, &mut tilenet);
                entities.delete(entity);
            }
        }
    }
}


impl<'a> specs::System<'a> for InputSys {
    type SystemData = (Read<'a, DeltaTime>,
                       Read<'a, GameConfig>,
                       ReadStorage<'a, PlayerInput>,
                       WriteStorage<'a, Jump>,
                       WriteStorage<'a, Vel>);

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
            if input.down {
                if !conf.gravity_on {
                    vel.transl.y -= conf.hori_acc * t;
                }
            }
        }
    }
}
