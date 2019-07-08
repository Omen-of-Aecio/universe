use crate::game::*;
use crate::mediators::does_line_collide_with_grid::*;
use bimap::BiMap;
use fast_logger::{InDebug, Logger};
use geometry::{grid2d::Grid, vec::Vec2};
use laminar::{Packet, SocketEvent};
use rand::Rng;
use rand_pcg::Pcg64Mcg;
use std::net::SocketAddr;
use std::time::Instant;

pub struct Server {
    pub logger: Logger<Log>,
    pub logic: ServerLogic,
    pub random: Pcg64Mcg,
    pub time: Instant,
    // Communication
    pub network: Socket,
    pub connections: BiMap<Id, SocketAddr>,
}
impl Server {
    pub fn new(logger: Logger<Log>) -> Server {
        let mut s = Server {
            logger: logger,
            logic: ServerLogic::default(),
            random: Pcg64Mcg::new(0),
            time: Instant::now(),
            //
            network: random_port_socket(),
            connections: BiMap::new(),
        };
        initialize_grid(&mut s.logic.grid);
        create_black_square_around_player(&mut s.logic.grid);
        s
    }
    pub fn tick_logic(&mut self) {
        self.update_network();
        self.logic
            .update_players(&mut self.random, &mut self.logger);
        self.logic.update_bullets();

        std::thread::sleep(std::time::Duration::new(0, 8_000_000));
    }

    fn fire_bullet(&mut self) {}
    fn update_network(&mut self) {
        // Handle incoming messages
        loop {
            self.network.manual_poll(self.time);
            match self.network.recv() {
                Some(SocketEvent::Packet(pkt)) => {
                    let msg = ClientMessage::deserialize(pkt.payload());
                    if let Ok(msg) = msg {
                        match msg {
                            ClientMessage::Join => {
                                info![self.logger, "server", "Received Join message"];
                                let id = self.logic.add_player();

                                self.connections.insert(id, pkt.addr());

                                self.network
                                    .send(Packet::reliable_unordered(
                                        pkt.addr(),
                                        ServerMessage::Welcome { your_id: id }.serialize(),
                                    ))
                                    .unwrap_or_else(|_| {
                                        error![
                                            self.logger,
                                            "server", "Failed to send Welcome packet"
                                        ];
                                    });
                            }
                            ClientMessage::Input {
                                commands,
                                mouse_pos,
                            } => {
                                let id = self.connections.get_by_right(&pkt.addr());
                                match id {
                                    Some(id) => {
                                        self.logic
                                            .players
                                            .iter_mut()
                                            .find(|player| player.id == *id)
                                            .map(|player| {
                                                for cmd in commands {
                                                    player.input.apply_command(cmd);
                                                }
                                                player.input.mouse_pos = mouse_pos;
                                            });
                                    }
                                    None => {
                                        error![
                                            self.logger,
                                            "server", "Unregistered client sent Input message"
                                        ];
                                    }
                                }
                            }
                        }
                    } else {
                        error![
                            self.logger,
                            "server", "Failed to deserialize an incoming message"
                        ];
                    }
                }
                Some(SocketEvent::Connect(_addr)) => {}
                Some(SocketEvent::Timeout(_addr)) => {} /*TODO*/
                None => break,
            }
        }
        // Send state updates
        let players: Vec<_> = self.logic.players.iter().map(|p| p.inner.clone()).collect();
        for cli_addr in self.connections.right_values() {
            self.network
                .send(Packet::unreliable(
                    *cli_addr,
                    ServerMessage::State {
                        players: players.clone(),
                    }
                    .serialize(),
                ))
                .unwrap();
        }
    }
}
#[derive(Default, Debug)]
pub struct ServerLogic {
    pub grid: Grid<(u8, u8, u8, u8)>,
    pub config: Config,
    pub players: Vec<ServerPlayer>,
    pub bullets: Vec<Bullet>,
    // ID counters
    player_id: Id,
    bullet_id: Id,
}
impl ServerLogic {
    pub fn add_player(&mut self) -> Id {
        let id = self.player_id;
        self.player_id += 1;
        let player = ServerPlayer {
            inner: PlayerData::new(id, 0, Vec2::null_vec()),
            input: UserInput::new(),
        };
        self.players.push(player);
        id
    }

    pub fn update_players(&mut self, random: &mut Pcg64Mcg, logger: &mut Logger<Log>) {
        for player in &mut self.players {
            // Physics
            if self.config.world.gravity_on {
                player.velocity += Vec2::new(0.0, self.config.world.gravity);
            }

            let on_ground = check_for_collision_and_move_player_according_to_movement_vector(
                &self.grid, player, logger,
            );
            let acc = accelerate_player_according_to_input(&player.input, &self.config, on_ground);
            player.velocity += acc;

            player.velocity = player.velocity.clamp(Vec2 {
                x: self.config.player.max_vel,
                y: self.config.player.max_vel,
            });
            if on_ground {
                player.velocity.x *= self.config.world.ground_fri;
            } else {
                player.velocity.x *= self.config.world.air_fri_x;
            }
            player.velocity.y *= self.config.world.air_fri_y;

            // Firing weapons
            if player.input.is_down(InputKey::LeftMouse) {
                let stats = player.curr_weapon.get_stats();
                for _ in 0..stats.bullet_count {
                    let angle =
                        Vec2::from(player.input.mouse_pos) - player.position - Vec2::new(5.0, 5.0);
                    let direction = angle.rotate(random.gen_range(-stats.spread, stats.spread));

                    let position = player.position + Vec2::new(5.0, 5.0);
                    let id = self.bullet_id;
                    self.bullet_id += 1;
                    self.bullets.push(Bullet {
                        direction: direction.normalize() * stats.speed,
                        position,
                        id,
                        ty: player.curr_weapon,
                    });
                }
            }
        }
    }

    pub fn update_bullets(&mut self) {
        for b in self.bullets.iter_mut() {
            let collision =
                collision_test(&[b.position], None, b.direction, &self.grid, |x| x.1 > 0);
            if let Some((xi, yi)) = collision {
                let area = b.ty.get_stats().destruction;
                for i in -area..=area {
                    for j in -area..=area {
                        let pos = (xi as i32 + i, yi as i32 + j);
                        let pos = (pos.0 as usize, pos.1 as usize);
                        self.grid.set(pos.0, pos.1, (0, 0, 0, 0));
                        // self.changed_tiles.push((pos.0, pos.1));
                        // TODO: Need to update changed_tiles on client (based on what chunks it
                        // receives prolly)
                    }
                }
            } else {
                b.position += b.direction;
            }
        }
    }
}

#[derive(Debug)]
pub struct ServerPlayer {
    inner: PlayerData,
    pub input: UserInput,
}

impl std::ops::Deref for ServerPlayer {
    type Target = PlayerData;
    fn deref(&self) -> &PlayerData {
        &self.inner
    }
}
impl std::ops::DerefMut for ServerPlayer {
    fn deref_mut(&mut self) -> &mut PlayerData {
        &mut self.inner
    }
}

/// Indexed by InputKey
#[derive(Debug)]
pub struct UserInput {
    keys: Vec<bool>,
    pub mouse_pos: (f32, f32),
}

impl UserInput {
    pub fn new() -> UserInput {
        UserInput {
            keys: vec![false; InputKey::LeftMouse as usize + 1],
            mouse_pos: (0.0, 0.0),
        }
    }
    pub fn apply_command(&mut self, cmd: InputCommand) {
        self.keys[cmd.key as usize] = cmd.is_pressed;
    }
    pub fn is_down(&self, key: InputKey) -> bool {
        self.keys[key as usize]
    }
}

/// Returns true if collision happened on y axis
fn check_for_collision_and_move_player_according_to_movement_vector(
    grid: &Grid<(u8, u8, u8, u8)>,
    player: &mut PlayerData,
    _logger: &mut Logger<Log>,
) -> bool {
    let movement = player.velocity.clone();
    let tl = Vec2 {
        x: player.position.x + 0.01,
        y: player.position.y + 0.01,
    };
    let tr = Vec2 {
        x: player.position.x + 9.99,
        y: player.position.y + 0.01,
    };
    let bl = Vec2 {
        x: player.position.x + 0.01,
        y: player.position.y + 9.99,
    };
    let br = Vec2 {
        x: player.position.x + 9.99,
        y: player.position.y + 9.99,
    };
    let mut collision_y = None;
    let ymove = Vec2 {
        x: 0.0,
        y: movement.y,
    };
    for i in 1..=10 {
        collision_y = collision_test(&[tl, tr, br, bl], Some(0.5), ymove / i as f32, grid, |x| {
            x.1 > 0
        });
        if collision_y.is_none() {
            player.position += ymove / i as f32;
            break;
        }
    }

    let tl = Vec2 {
        x: player.position.x + 0.01,
        y: player.position.y + 0.01,
    };
    let tr = Vec2 {
        x: player.position.x + 9.99,
        y: player.position.y + 0.01,
    };
    let bl = Vec2 {
        x: player.position.x + 0.01,
        y: player.position.y + 9.99,
    };
    let br = Vec2 {
        x: player.position.x + 9.99,
        y: player.position.y + 9.99,
    };
    let mut collision_x = None;
    let xmove = Vec2 {
        x: movement.x,
        y: 0.0,
    };
    for i in 1..=10 {
        collision_x = collision_test(&[tl, tr, br, bl], Some(0.5), xmove / i as f32, grid, |x| {
            x.1 > 0
        });
        if collision_x.is_none() {
            player.position += xmove / i as f32;
            break;
        }
    }
    if collision_x.is_some() {
        player.velocity.x = 0.0;
    }
    if collision_y.is_some() {
        player.velocity.y = 0.0;
    }
    collision_y.is_some()
}
