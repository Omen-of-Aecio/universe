use crate::glocals::*;
pub use failure::Error;
use fast_logger::{error, info, Logger};
use geometry::vec::Vec2;
use geometry::{boxit::Boxit, grid2d::Grid};
use laminar::Socket;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::{time::Instant, vec::Vec};
use vxdraw::{self, *};

pub mod client;
pub use client::*;
pub mod server;
pub use server::*;

pub type Id = u32;

pub struct Main {
    pub cli: Option<Client>,
    pub srv: Option<Server>,
}
impl Main {
    pub fn new(mut cli: Option<Client>, srv: Option<Server>, mut logger: Logger<Log>) -> Main {
        if let (Some(cli), Some(srv)) = (&mut cli, &srv) {
            if let Err(e) = cli.connect_to_server(srv.network.local_addr().unwrap()) {
                info![logger, "main", "Failed to connect to server"; "err" => e];
            }
        }
        Main { cli, srv }
    }
    pub fn entry_point(&mut self) {
        loop {
            if let Some(ref mut cli) = self.cli {
                cli.time = Instant::now();
                cli.tick_logic();
                if cli.logic.should_exit {
                    break;
                }
            }
            if let Some(ref mut srv) = self.srv {
                srv.time = Instant::now();
                srv.tick_logic();
            }
        }
    }
}

// ---

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Bullet {
    pub direction: Vec2,
    pub position: Vec2,
    pub id: u32,
    pub ty: Weapon,
}
impl Bullet {
    pub fn get_stats(&self) -> WeaponStats {
        self.ty.get_stats()
    }
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct PlayerData {
    pub position: Vec2,
    pub velocity: Vec2,
    pub id: Id,
    pub curr_weapon: Weapon,
    pub curr_weapon_cooldown: usize,
    /// Reality in which the player resides. Reality signifies the colour of the air in which the
    /// player resides.
    pub reality: u32,
}
impl PlayerData {
    pub fn new(id: Id, reality: u32, position: Vec2) -> PlayerData {
        PlayerData {
            position,
            velocity: Vec2::null_vec(),
            id,
            curr_weapon: Weapon::Hellfire,
            curr_weapon_cooldown: 0,
            reality,
        }
    }
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone, Copy)]
pub enum Weapon {
    Hellfire,
    Ak47,
}

impl Weapon {
    pub fn get_stats(&self) -> WeaponStats {
        match *self {
            Weapon::Hellfire => WeaponStats {
                width: 10,
                height: 6,
                animation_block_begin: (0.0, 0.0),
                animation_block_end: (1.0, 53.0 / 60.0),
                sprite_width: 6.8,
                sprite_height: 0.9,
                destruction: 3,
                bullet_count: 1,
                spread: 0.3,
                speed: 1.0,
            },
            Weapon::Ak47 => {
                (WeaponStats {
                    width: 1,
                    height: 1,
                    animation_block_begin: (0.0, 54.0 / 60.0),
                    animation_block_end: (4.0 / 679.0, 58.0 / 60.0),
                    sprite_width: 0.5,
                    sprite_height: 0.5,
                    destruction: 1,
                    bullet_count: 1,
                    spread: 0.1,
                    speed: 2.0,
                })
            }
        }
    }
}

impl Default for Weapon {
    fn default() -> Self {
        Weapon::Hellfire
    }
}
pub struct WeaponStats {
    pub width: usize,
    pub height: usize,
    pub animation_block_begin: (f32, f32),
    pub animation_block_end: (f32, f32),
    pub sprite_width: f32,
    pub sprite_height: f32,
    pub destruction: i32,
    pub bullet_count: u8,
    pub spread: f32,
    pub speed: f32,
}

#[derive(Copy, Clone)]
pub struct Vertex {
    pub pos: [f32; 2],
}

// Not sure where to put this. Helper for laminar::Socket
pub fn random_port_socket() -> Socket {
    let loopback = Ipv4Addr::new(127, 0, 0, 1);
    let socket = SocketAddrV4::new(loopback, 0);
    Socket::bind(socket).unwrap() // TODO laminar error not compatible with failure?
}

static FIREBALLS: &dyntex::ImgData =
    &dyntex::ImgData::PNGBytes(include_bytes!["../../assets/images/bullets.png"]);
static WEAPONS: &dyntex::ImgData =
    &dyntex::ImgData::PNGBytes(include_bytes!["../../assets/images/weapons.png"]);

pub fn initialize_grid(s: &mut Grid<(u8, u8, u8, u8)>) {
    s.resize(1000, 1000);
}

pub fn create_black_square_around_player(s: &mut Grid<(u8, u8, u8, u8)>) {
    for (i, j) in Boxit::with_center((100, 100), (500, 300)) {
        s.set(i, j, (0, 0, 0, 0));
    }
}

pub fn accelerate_player_according_to_input(
    inp: &UserInput,
    conf: &WorldConfig,
    on_ground: bool,
) -> Vec2 {
    let dy = if inp.is_down(InputKey::Up) {
        if conf.gravity_on {
            if on_ground {
                -conf.player.jump_acc
            } else {
                0.0
            }
        } else {
            -conf.player.acc
        }
    } else if inp.is_down(InputKey::Down) {
        conf.player.acc
    } else {
        0.0
    };
    let dx = if inp.is_down(InputKey::Left) {
        -conf.player.acc
    } else if inp.is_down(InputKey::Right) {
        conf.player.acc
    } else {
        0.0
    };
    Vec2 {
        x: dx as f32,
        y: dy as f32,
    } / if inp.is_down(InputKey::LShift) {
        3.0
    } else {
        1.0
    }
}

// TODO: split into client and server
/*
fn fire_bullets(
    s: &mut ClientLogic,
    graphics: &mut Option<Graphics>,
    random: &mut rand_pcg::Pcg64Mcg,
) {
    if s.input.is_left_mouse_button_down() {
        if s.current_weapon_cooldown == 0 {
            s.current_weapon_cooldown = match s.current_weapon {
                Weapon::Hellfire => 5,
                Weapon::Ak47 => 2,
            }
        } else {
            s.current_weapon_cooldown -= 1;
            return;
        }

        let weapon = &s.current_weapon;

        let spread = if weapon == &Weapon::Hellfire {
            0.3
        } else {
            0.1
        };

        let (
            width,
            height,
            animation_block_begin,
            animation_block_end,
            sprite_width,
            sprite_height,
            destruction,
            bullet_count,
            speed,
        ) = match weapon {
            Weapon::Hellfire => (10, 6, (0.0, 0.0), (1.0, 53.0 / 60.0), 6.8, 0.9, 3, 1, 1.0),
            Weapon::Ak47 => (
                1,
                1,
                (0.0, 54.0 / 60.0),
                (4.0 / 679.0, 58.0 / 60.0),
                0.5,
                0.5,
                1,
                1,
                2.0,
            ),
        };

        for _ in 0..bullet_count {
            let direction = if let Some(ref mut graphics) = graphics {
                if let Some(player) = s.players.get(0) {
                    let mouse_in_world =
                        graphics.windowing.to_world_coords(s.input.get_mouse_pos());
                    let angle = (Vec2::from(mouse_in_world) - player.position - PLAYER_CENTER);
                    angle.rotate(random.gen_range(-spread, spread))
                } else {
                    Vec2 { x: 1.0, y: 0.0 }
                }
            } else {
                Vec2 { x: 1.0, y: 0.0 }
            };

            let handle = if let Some(ref mut graphics) = graphics {
                Some(
                    graphics.windowing.dyntex().add(
                        &graphics.bullets_texture,
                        vxdraw::dyntex::Sprite::new()
                            .width(sprite_width)
                            .height(sprite_height)
                            .scale(3.0)
                            .origin((-sprite_width / 2.0, sprite_height / 2.0))
                            .rotation(Rad(-direction.angle() + std::f32::consts::PI)),
                    ),
                )
            } else {
                None
            };

            let position = s.players.get(0).map_or(Vec2 { x: 0.0, y: 0.0 }, |x| {
                x.position + Vec2 { x: 5.0, y: 5.0 }
            });
            s.bullets.push(ClientBullet {
                direction: direction.normalize() * speed,
                position,
                destruction,

                animation_sequence: 0,
                animation_block_begin,
                animation_block_end,
                height,
                width,
                current_uv_begin: (0.0, 0.0),
                current_uv_end: (0.0, 0.0),
                handle,
            });
        }
    }
}
*/

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Client;
    use crate::mediators::testtools::*;
    use fast_logger::Logger;

    #[test]
    fn basic_setup_and_teardown() {
        crate::game::Server::new(fast_logger::Logger::spawn_void());
    }

    #[test]
    fn basic_setup_gsh() {
        let mut main = Client::new(Logger::spawn_void(), GraphicsSettings::DisableGraphics);
        spawn_gameshell(&mut main);
        assert![main.threads.game_shell_channel.is_some()];
        assert_eq!["6", gsh(&mut main, "+ 1 2 3")];
    }

    #[test]
    fn gsh_change_gravity() {
        let mut cli = Client::new(Logger::spawn_void(), GraphicsSettings::DisableGraphics);
        spawn_gameshell(&mut cli);
        assert_eq![
            "Set gravity value",
            gsh(&mut cli, "config gravity set y 1.23")
        ];
        cli.tick_logic();
        assert_eq![1.23, cli.logic.config.gravity];
    }

    #[test]
    fn gsh_change_gravity_synchronous() {
        let mut cli = Client::new(Logger::spawn_void(), GraphicsSettings::DisableGraphics);
        let mut main = Main::new(Some(cli), None, Logger::spawn_void());
        spawn_gameshell(main.cli.as_mut().unwrap());
        assert_eq![
            "Set gravity value",
            gsh_synchronous(&mut main, "config gravity set y 1.23", |main| main
                .cli
                .as_mut()
                .unwrap()
                .tick_logic())
        ];
        assert_eq![1.23, main.cli.as_mut().unwrap().logic.config.gravity];
    }

    #[test]
    fn gsh_get_fps() {
        let mut cli = Client::new(Logger::spawn_void(), GraphicsSettings::DisableGraphics);
        spawn_gameshell(&mut cli);
        let mut main = Main::new(Some(cli), None, Logger::spawn_void());
        assert_eq![
            "0",
            gsh_synchronous(&mut main, "config fps get", |main| main
                .cli
                .as_mut()
                .unwrap()
                .tick_logic())
        ];

        gsh(main.cli.as_mut().unwrap(), "config fps set 1.23");
        main.cli.as_mut().unwrap().tick_logic();

        assert_eq![
            "1.23",
            gsh_synchronous(&mut main, "config fps get", |main| main
                .cli
                .as_mut()
                .unwrap()
                .tick_logic())
        ];
    }
}
