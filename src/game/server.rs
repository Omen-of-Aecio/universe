use crate::game::*;
use crate::mediators::does_line_collide_with_grid::*;
use bimap::BiMap;
use cgmath::*;
use fast_logger::{GenericLogger, Logger};
use geometry::{grid2d::Grid, vec::Vec2};
use laminar::{Packet, SocketEvent};
use rand::Rng;
use rand_pcg::Pcg64Mcg;
use std::net::SocketAddr;
use std::time::Instant;

const WORLD_WIDTH: usize = 1000;
const WORLD_HEIGHT: usize = 1000;
const WORLD_SEED: [f32; 3] = [0.0, 0.0, 0.0];

fn generate_world(w: usize, h: usize, seed: [f32; 3], mut logger: Logger<Log>) -> Grid<Reality> {
    let mut grid = Grid::default();
    grid.resize(w, h);
    logger.info("Initializing graphics");
    let mut windowing = VxDraw::new(
        logger.clone_add_context("vxdraw").to_compatibility(),
        ShowWindow::Headless1k,
    );

    {
        static BACKGROUND: &dyntex::ImgData =
            &dyntex::ImgData::PNGBytes(include_bytes!["../../assets/images/terrabackground.png"]);
        let background = windowing.dyntex().add_layer(
            BACKGROUND,
            &dyntex::LayerOptions::new()
                .depth(true)
                .fixed_perspective(Matrix4::identity()),
        );
        windowing.dyntex().add(&background, dyntex::Sprite::new());
    }

    let mut strtex = windowing.strtex();

    let tex = strtex.add_layer(&strtex::LayerOptions::new().width(w).height(h).depth(false));

    strtex.fill_with_perlin_noise(&tex, seed);
    strtex.read(&tex, |x, pitch| {
        for j in 0..w {
            for i in 0..h {
                grid.set(i, j, x[i + j * pitch].0);
            }
        }
    });
    grid
}

pub struct Server {
    pub logger: Logger<Log>,
    pub logic: ServerLogic,
    pub random: Pcg64Mcg,
    pub time: Instant,
    pub config: ServerConfig,
    // Communication
    pub network: Socket,
    pub connections: BiMap<Id, SocketAddr>,
}

impl Server {
    pub fn new(logger: Logger<Log>) -> Server {
        let mut cfg = laminar::Config::default();
        cfg.receive_buffer_max_size = cfg.max_packet_size;
        let mut s = Server {
            logger: logger.clone(),
            logic: ServerLogic::default(),
            random: Pcg64Mcg::new(0),
            time: Instant::now(),
            config: Default::default(),
            //
            network: random_port_socket(cfg),
            connections: BiMap::new(),
        };
        s.logic.grid = generate_world(WORLD_WIDTH, WORLD_HEIGHT, WORLD_SEED, logger.clone());
        create_black_square_around_player(&mut s.logic.grid);

        s
    }
    /// Assigns `config.server` to `self.config` and `config.world` to `self.logic.config`.
    pub fn apply_config(&mut self, config: Config) {
        let (s, w) = (config.server, config.world);
        self.config = s;
        self.logic.config = w;
    }

    pub fn tick_logic(&mut self) {
        self.update_network();
        self.logic
            .update_players(&mut self.random, &mut self.logger);
        self.logic.update_bullets();

        std::thread::sleep(std::time::Duration::new(0, 8_000_000));
    }

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
                                info![self.logger, "Received Join message"];
                                let id = self.logic.add_player();

                                self.connections.insert(id, pkt.addr());

                                self.network
                                    .send(Packet::reliable_unordered(
                                        pkt.addr(),
                                        ServerMessage::Welcome {
                                            your_id: id,
                                            world_width: WORLD_WIDTH,
                                            world_height: WORLD_HEIGHT,
                                            world_seed: WORLD_SEED,
                                        }
                                        .serialize(),
                                    ))
                                    .unwrap_or_else(|_| {
                                        error![self.logger, "Failed to send Welcome packet"];
                                    });
                            }
                            ClientMessage::Input {
                                commands,
                                mouse_pos,
                            } => {
                                let id = self.connections.get_by_right(&pkt.addr());
                                match id {
                                    Some(id) => {
                                        if let Some(player) = self
                                            .logic
                                            .players
                                            .iter_mut()
                                            .find(|player| player.id == *id)
                                        {
                                            for cmd in commands {
                                                player.input.apply_command(cmd);
                                            }
                                            player.input.mouse_pos = mouse_pos;
                                        }
                                    }
                                    None => {
                                        error![
                                            self.logger,
                                            "Unregistered client sent Input message"
                                        ];
                                    }
                                }
                            }
                        }
                    } else {
                        error![self.logger, "Failed to deserialize an incoming message"];
                    }
                }
                Some(SocketEvent::Connect(_addr)) => {}
                Some(SocketEvent::Timeout(_addr)) => {} /*TODO*/
                None => break,
            }
        }
        // Send state updates
        let players: Vec<_> = self.logic.players.iter().map(|p| p.inner.clone()).collect();
        let state_data = ServerMessage::State {
            players: players.clone(),
            bullets: self.logic.bullets.clone(),
        }
        .serialize();
        let delta_data = ServerMessage::DeltaState {
            removed: self.logic.removed.clone(),
            grid_changes: self.logic.grid_changes.clone(),
        }
        .serialize();
        for cli_addr in self.connections.right_values() {
            self.network
                .send(Packet::unreliable(*cli_addr, state_data.clone()))
                .unwrap();
            self.network
                .send(Packet::reliable_unordered(*cli_addr, delta_data.clone()))
                .unwrap();
        }

        // Cleanup / reset state
        self.logic.grid_changes = Vec::new();
        self.logic.removed = Vec::new();
    }
}

#[derive(Default, Debug)]
pub struct ServerLogic {
    pub grid: Grid<Reality>,
    pub players: Vec<ServerPlayer>,
    pub bullets: Vec<Bullet>,
    pub config: WorldConfig,
    // ID counters
    player_id: Id,
    bullet_id: Id,
    //
    grid_changes: Vec<(u32, u32, Reality)>,
    removed: Vec<(Id, EntityType)>,
}

impl ServerLogic {
    pub fn new(config: WorldConfig) -> Self {
        ServerLogic {
            config,
            ..Default::default()
        }
    }

    pub fn add_player(&mut self) -> Id {
        let id = self.player_id;
        self.player_id += 1;
        let player = ServerPlayer {
            inner: PlayerData::new(id, 0, Vec2::null_vec()),
            input: UserInput::default(),
        };
        self.players.push(player);
        id
    }

    pub fn update_players(&mut self, random: &mut Pcg64Mcg, logger: &mut Logger<Log>) {
        for player in &mut self.players {
            update_player(
                &mut player.inner,
                &mut player.input,
                &self.config,
                random,
                &self.grid,
                logger,
            );

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
        let mut to_remove = Vec::new();
        for (idx, b) in self.bullets.iter_mut().enumerate() {
            let collision =
                collision_test(&[b.position], None, b.direction, &self.grid, |x| *x > 0);
            if let Some((xi, yi)) = collision {
                to_remove.push(idx);
                let area = b.ty.get_stats().destruction;
                for i in -area..=area {
                    for j in -area..=area {
                        let pos = (xi as i32 + i, yi as i32 + j);
                        let pos = (pos.0 as usize, pos.1 as usize);
                        self.grid.set(pos.0, pos.1, 0);
                        self.grid_changes.push((pos.0 as u32, pos.1 as u32, 0));
                    }
                }
            } else {
                b.position += b.direction;
            }
        }

        // Remove bullets
        use std::cmp::Ordering;
        to_remove.sort_by(|x, y| {
            if *x < *y {
                Ordering::Greater
            } else if *x == *y {
                Ordering::Equal
            } else {
                Ordering::Less
            }
        });

        for idx in to_remove {
            let bullet = self.bullets.swap_remove(idx);
            self.removed.push((bullet.id, EntityType::Bullet));
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

/// Returns true if collision happened on y axis
fn check_for_collision_and_move_player_according_to_movement_vector(
    grid: &Grid<Reality>,
    player: &mut PlayerData,
    _logger: &mut Logger<Log>,
) -> bool {
    let movement = player.velocity;
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
            *x > 0
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
            *x > 0
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
