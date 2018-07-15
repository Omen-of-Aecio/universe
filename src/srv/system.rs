use specs;
use specs::{Fetch, ReadStorage, WriteStorage, Join};
use collision::player_move;
use component::*;
use tilenet::TileNet;
use global::Tile;
use srv::game::GameConfig;
use geometry::Vec2;

////////////
// Server

pub struct JumpSys;
/// For Player <-> TileNet collision
pub struct MoveSys;
pub struct InputSys;

// (TODO extra friction when on ground?)

impl<'a> specs::System<'a> for JumpSys {
    type SystemData = (Fetch<'a, GameConfig>,
                       WriteStorage<'a, Jump>,
                       WriteStorage<'a, Vel>);
    fn run(&mut self, data: Self::SystemData) {
        let (config, mut jump, mut vel) = data;
        for (jump, vel) in (&mut jump, &mut vel).join() {
            // Jump
            let acc = jump.tick(config.srv_frame_duration);
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
    type SystemData = (Fetch<'a, TileNet<Tile>>,
                       Fetch<'a, GameConfig>, // gravity
                       ReadStorage<'a, Player>,
                       WriteStorage<'a, Pos>,
                       WriteStorage<'a, Vel>,
                       WriteStorage<'a, Force>,
                       ReadStorage<'a, Shape>,
                       ReadStorage<'a, Color>);

    fn run(&mut self, data: Self::SystemData) {
        let (tilenet, game_conf, player, mut pos, mut vel, mut force, shape, color) = data;
        let gravity = if game_conf.gravity_on { game_conf.gravity } else { Vec2::null_vec() };

        for (_, pos, vel, force, shape, color) in
            (&player, &mut pos, &mut vel, &mut force, &shape, &color).join() {
                player_move(pos, vel, force, shape, color, &tilenet, gravity);
        }
    }
}


impl<'a> specs::System<'a> for InputSys {
    type SystemData = (Fetch<'a, GameConfig>,
                       ReadStorage<'a, PlayerInput>,
                       WriteStorage<'a, Jump>,
                       WriteStorage<'a, Vel>);

    fn run(&mut self, data: Self::SystemData) {
        let (conf, input, mut jump, mut vel) = data;
        for (input, jump, vel) in (&input, &mut jump, &mut vel).join() {
            if input.left {
                vel.transl.x -= conf.hori_acc;
            }
            if input.right {
                vel.transl.x += conf.hori_acc;

            }
            if input.up {
                if conf.gravity_on {
                    if !jump.is_active() {
                        *jump = Jump::new_active(conf.jump_duration, conf.jump_acc);
                    }
                } else {
                    vel.transl.y += conf.hori_acc;
                }
            }
            if input.down {
                if !conf.gravity_on {
                    vel.transl.y -= conf.hori_acc;
                }
            }
        }
    }
}
