use specs;
use specs::{Fetch, ReadStorage, WriteStorage, Join};
use collision::player_move;
use component::*;
use tile_net::TileNet;
use global::Tile;
use srv::game::GameConfig;

////////////
// Server

pub struct JumpSys;
/// For Player <-> TileNet collision
pub struct MoveSys;
pub struct InputSys;

const JUMP_DURATION: u32 = 4;
const JUMP_DELAY: u32 = 20; // Delay before you can jump again
const JUMP_ACC: f32 = 3.0;
// (TODO extra friction when on ground?)

impl<'a> specs::System<'a> for JumpSys {
    type SystemData = (WriteStorage<'a, Jump>,
                       WriteStorage<'a, Vel>);
    fn run(&mut self, data: Self::SystemData) {
        let (mut jump, mut vel) = data;
        for (jump, vel) in (&mut jump, &mut vel).join() {
            // Jump
            let acc = jump.tick();
            let progress = jump.get_progress();
            if let Some(acc) = acc {
                vel.transl.y += acc;
            }
            if let Some(progress) = progress {
                if progress > JUMP_DELAY {
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

        for (_, pos, vel, force, shape, color) in
            (&player, &mut pos, &mut vel, &mut force, &shape, &color).join() {
                println!("MoveSys vel: {:?}", vel.transl);
                player_move(pos, vel, force, shape, color, &tilenet, game_conf.gravity);
        }
    }
}

const ACCELERATION: f32 = 0.35;

impl<'a> specs::System<'a> for InputSys {
    type SystemData = (Fetch<'a, GameConfig>,
                       ReadStorage<'a, PlayerInput>,
                       WriteStorage<'a, Jump>,
                       WriteStorage<'a, Vel>);

    fn run(&mut self, data: Self::SystemData) {
        let (game_conf, input, mut jump, mut vel) = data;
        for (input, jump, vel) in (&input, &mut jump, &mut vel).join() {
            if input.left {
                vel.transl.x -= ACCELERATION;
            }
            if input.right {
                vel.transl.x += ACCELERATION;

            }
            if input.up {
                if game_conf.gravity_on {
                    if !jump.is_active() {
                        *jump = Jump::new_active(JUMP_DURATION, JUMP_ACC);
                    }
                } else {
                    vel.transl.y += ACCELERATION;
                }
            }
            if input.down {
                if !game_conf.gravity_on {
                    vel.transl.y -= ACCELERATION;
                }
            }
        }
    }
}
