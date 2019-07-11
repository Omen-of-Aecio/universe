use super::*;
use geometry::{cam::Camera, grid2d::Grid, vec::Vec2};
use indexmap::IndexMap;
use input;
use laminar::{Packet, Socket, SocketEvent};
use rand_pcg::Pcg64Mcg;
use rodio;
use std::net::SocketAddr;

use cgmath::*;
use fast_logger::{info, GenericLogger, Logger};
use input::Input;
use std::time::Instant;
use winit::{VirtualKeyCode as Key, *};

static PLAYER_CENTER: Vec2 = Vec2 { x: 5.0, y: 5.0 };

pub struct Client {
    pub audio: Option<rodio::Sink>,
    pub graphics: Option<Graphics>,
    pub logger: Logger<Log>,
    pub logic: ClientLogic,
    pub config: ClientConfig,
    pub network: Socket,
    pub random: Pcg64Mcg,
    pub threads: Threads,
    pub time: Instant,
    pub input: input::Input,
    pub server: Option<SocketAddr>,
}

#[derive(Default)]
pub struct ClientLogic {
    pub should_exit: bool,

    pub grid: Grid<(u8, u8, u8, u8)>,
    pub config: WorldConfig,
    pub players: IndexMap<Id, ClientPlayer>,
    pub bullets: IndexMap<Id, ClientBullet>,
    pub self_id: Id,

    pub cam: Camera,
    pub you: u32,

    pub white_base: Vec2,
    pub black_base: Vec2,

    // Extra graphics data (for debugging/visualization)
    pub vectors: Vec<(Vec2, Vec2)>,

    pub cam_mode: CameraMode,

    pub changed_tiles: Vec<(usize, usize)>,
    pub bullets_added: Vec<Vec2>,
}

#[derive(Default)]
pub struct ClientPlayer {
    inner: PlayerData,
    pub weapon_sprite: Option<vxdraw::dyntex::Handle>,
}
impl std::ops::Deref for ClientPlayer {
    type Target = PlayerData;
    fn deref(&self) -> &PlayerData {
        &self.inner
    }
}
impl std::ops::DerefMut for ClientPlayer {
    fn deref_mut(&mut self) -> &mut PlayerData {
        &mut self.inner
    }
}

pub struct ClientBullet {
    /// Holds the logical data
    inner: Bullet,
    pub handle: vxdraw::dyntex::Handle,

    pub animation_sequence: usize,
    pub current_uv_begin: (f32, f32),
    pub current_uv_end: (f32, f32),
}
impl std::ops::Deref for ClientBullet {
    type Target = Bullet;
    fn deref(&self) -> &Bullet {
        &self.inner
    }
}

/* Should go, together with some logic, to some camera module (?) */
#[derive(Copy, Clone, PartialEq)]
pub enum CameraMode {
    Interactive,
    FollowPlayer,
}

pub struct Graphics {
    pub basic_text: vxdraw::text::Handle,
    pub player_quads: Vec<vxdraw::quads::Handle>,
    pub bullets_texture: vxdraw::dyntex::Layer,
    pub weapons_texture: vxdraw::dyntex::Layer,
    pub grid: vxdraw::strtex::Layer,
    pub windowing: vxdraw::VxDraw,
}

// Not sure where to put this. Helper for laminar::Socket
fn random_port_socket() -> Socket {
    let loopback = Ipv4Addr::new(127, 0, 0, 1);
    let socket = SocketAddrV4::new(loopback, 0);
    Socket::bind(socket).unwrap() // TODO laminar error not compatible with failure?
}

// ---

impl Default for CameraMode {
    fn default() -> CameraMode {
        CameraMode::Interactive
    }
}

#[derive(PartialEq)]
pub enum GraphicsSettings {
    EnableGraphics,
    DisableGraphics,
}

impl Client {
    pub fn new(logger: Logger<Log>, graphics: GraphicsSettings) -> Client {
        let mut s = Client {
            audio: None,
            graphics: None,
            logger: logger,
            logic: ClientLogic::default(),
            network: random_port_socket(),
            random: Pcg64Mcg::new(0),
            threads: Threads::default(),
            time: Instant::now(),
            input: Input::default(),
            server: None,
            config: Default::default(),
        };

        s.logic.cam.zoom = 0.01;
        if graphics == GraphicsSettings::EnableGraphics {
            s.maybe_initialize_graphics();
        }
        initialize_grid(&mut s.logic.grid);
        create_black_square_around_player(&mut s.logic.grid);

        let port = s.network.local_addr().unwrap().port();
        info![s.logger, "client", "Listening on port"; "port" => port];
        s
    }
    /// Assigns `config.client` to `self.config` and `config.world` to `self.logic.config`.
    pub fn apply_config(&mut self, config: Config) {
        let (s, w) = (config.client, config.world);
        self.config = s;
        self.logic.config = w;
    }

    fn get_me(&mut self) -> Option<&mut ClientPlayer> {
        let id = self.logic.self_id;
        self.logic.players.get_mut(&id)
    }
    /// Sends a Join request to the server at `addr`.
    /// Note that completion of the handshake takes place in `self.tick_logic()`.
    pub fn connect_to_server(&mut self, addr: SocketAddr) -> Result<(), Error> {
        self.network
            .send(Packet::reliable_unordered(
                addr,
                ClientMessage::Join.serialize(),
            ))
            .unwrap(); /* TODO!! ? operator doesn't work here */
        info![self.logger, "client", "Sent Join"];
        Ok(())
    }

    pub fn tick_logic(&mut self) {
        self.update_network();

        toggle_camera_mode(self);
        self.input.prepare_for_next_frame();
        if let Some(ref mut graphics) = self.graphics {
            process_input(&mut self.input, &mut graphics.windowing);
        }
        update_bullets_uv(&mut self.logic);
        std::thread::sleep(std::time::Duration::new(0, 8_000_000));

        set_gravity(self);

        if let Some(Ok(msg)) = self
            .threads
            .game_shell_channel
            .as_mut()
            .map(|x| x.try_recv())
        {
            (msg)(self);
        }

        handle_mouse_scroll(self);

        // fire_bullets(&mut self.logic, &mut self.graphics, &mut self.random);

        update_graphics(self);

        draw_graphics(self);
    }
    fn update_network(&mut self) {
        // Process incoming messages
        loop {
            self.network.manual_poll(self.time);
            match self.network.recv() {
                Some(SocketEvent::Packet(pkt)) => {
                    let msg = ServerMessage::deserialize(pkt.payload());
                    if let Ok(msg) = msg {
                        match msg {
                            ServerMessage::Welcome { your_id } => {
                                self.server = Some(pkt.addr());
                                self.logic.self_id = your_id;
                                info![self.logger, "client", "Received Welcome message!"];
                            }
                            ServerMessage::State { players, bullets } => {
                                for player in players {
                                    if self.logic.players.contains_key(&player.id) {
                                        // Update existing player
                                        if let Some(p) = self.logic.players.get_mut(&player.id) {
                                            p.inner = player;
                                        }
                                    } else {
                                        // Create new player
                                        let id = player.id;
                                        let new = ClientPlayer {
                                            inner: player,
                                            weapon_sprite: None,
                                        };
                                        self.logic.players.insert(id, new);
                                    }
                                }
                                for bullet in bullets {
                                    if self.logic.bullets.contains_key(&bullet.id) {
                                        // Update existing bullet
                                        if let Some(b) = self.logic.bullets.get_mut(&bullet.id) {
                                            b.inner = bullet;
                                        }
                                    } else {
                                        // Create new bullet

                                        let handle = if let Some(ref mut graphics) = self.graphics {
                                            let stats = bullet.get_stats();
                                            Some(
                                                graphics.windowing.dyntex().add(
                                                    &graphics.bullets_texture,
                                                    vxdraw::dyntex::Sprite::new()
                                                        .width(stats.sprite_width)
                                                        .height(stats.sprite_height)
                                                        .scale(3.0)
                                                        .origin((
                                                            -stats.sprite_width / 2.0,
                                                            stats.sprite_height / 2.0,
                                                        ))
                                                        .rotation(Rad(-bullet.direction.angle()
                                                            + std::f32::consts::PI)),
                                                ),
                                            )
                                        } else {
                                            None
                                        };

                                        let id = bullet.id;
                                        let new = ClientBullet {
                                            inner: bullet,
                                            handle: handle.unwrap(), // TODO: make optional here aswell?

                                            animation_sequence: 0,
                                            current_uv_begin: (0.0, 0.0),
                                            current_uv_end: (0.0, 0.0),
                                        };
                                        self.logic.bullets.insert(id, new);
                                    }
                                }
                            }
                            ServerMessage::DeltaState { removed: _ } => {
                                // TODO
                            }
                        }
                    } else {
                        error![
                            self.logger,
                            "client", "Failed to deserialize an incoming message"
                        ];
                    }
                }
                Some(SocketEvent::Connect(_addr)) => {}
                Some(SocketEvent::Timeout(_addr)) => {}
                None => break,
            }
        }
        // Send input to server
        if let Some(addr) = self.server {
            self.network
                .send(Packet::reliable_ordered(
                    addr,
                    self.collect_input().serialize(),
                    None,
                ))
                .unwrap();
        }
    }
    fn collect_input(&self) -> ClientMessage {
        let mut commands = Vec::new();
        if self.input.is_key_toggled_down(Key::Down) {
            commands.push(InputCommand {
                is_pressed: true,
                key: InputKey::Down,
            });
        } else if self.input.is_key_toggled_up(Key::Down) {
            commands.push(InputCommand {
                is_pressed: false,
                key: InputKey::Down,
            });
        }
        if self.input.is_key_toggled_down(Key::Up) {
            commands.push(InputCommand {
                is_pressed: true,
                key: InputKey::Up,
            });
        } else if self.input.is_key_toggled_up(Key::Up) {
            commands.push(InputCommand {
                is_pressed: false,
                key: InputKey::Up,
            });
        }
        if self.input.is_key_toggled_down(Key::Left) {
            commands.push(InputCommand {
                is_pressed: true,
                key: InputKey::Left,
            });
        } else if self.input.is_key_toggled_up(Key::Left) {
            commands.push(InputCommand {
                is_pressed: false,
                key: InputKey::Left,
            });
        }
        if self.input.is_key_toggled_down(Key::Right) {
            commands.push(InputCommand {
                is_pressed: true,
                key: InputKey::Right,
            });
        } else if self.input.is_key_toggled_up(Key::Right) {
            commands.push(InputCommand {
                is_pressed: false,
                key: InputKey::Right,
            });
        }
        if self.input.is_key_toggled_down(Key::LShift) {
            commands.push(InputCommand {
                is_pressed: true,
                key: InputKey::LShift,
            });
        } else if self.input.is_key_toggled_up(Key::LShift) {
            commands.push(InputCommand {
                is_pressed: false,
                key: InputKey::LShift,
            });
        }
        if self.input.is_left_mouse_button_toggled() {
            if self.input.is_left_mouse_button_down() {
                commands.push(InputCommand {
                    is_pressed: true,
                    key: InputKey::LeftMouse,
                });
            } else {
                commands.push(InputCommand {
                    is_pressed: false,
                    key: InputKey::LeftMouse,
                });
            }
        }

        let mouse_pos = match self.graphics {
            Some(ref graphics) => graphics
                .windowing
                .to_world_coords(self.input.get_mouse_pos()),
            None => (0.0, 0.0),
        };

        ClientMessage::Input {
            commands,
            mouse_pos,
        }
    }

    fn maybe_initialize_graphics(&mut self) {
        self.logger.info("cli", "Initializing graphics");
        let mut windowing = VxDraw::new(self.logger.clone().to_compatibility(), ShowWindow::Enable);

        {
            static BACKGROUND: &dyntex::ImgData = &dyntex::ImgData::PNGBytes(include_bytes![
                "../../assets/images/terrabackground.png"
            ]);
            let background = windowing.dyntex().add_layer(
                BACKGROUND,
                &dyntex::LayerOptions::new()
                    .depth(true)
                    .fixed_perspective(Matrix4::identity()),
            );
            windowing.dyntex().add(&background, dyntex::Sprite::new());
        }

        let mut strtex = windowing.strtex();

        let tex = strtex.add_layer(
            &strtex::LayerOptions::new()
                .width(1000)
                .height(1000)
                .depth(false),
        );
        self.logic.grid.resize(1000, 1000);

        strtex.fill_with_perlin_noise(&tex, [0.0, 0.0, 0.0]);
        let grid = &mut self.logic.grid;
        strtex.read(&tex, |x, pitch| {
            for j in 0..1000 {
                for i in 0..1000 {
                    grid.set(i, j, x[i + j * pitch]);
                }
            }
        });
        strtex.add(
            &tex,
            vxdraw::strtex::Sprite::new()
                .width(1000.0)
                .height(1000.0)
                .translation((500.0, 500.0)),
        );
        let layer = windowing
            .quads()
            .add_layer(&vxdraw::quads::LayerOptions::default());
        let handle = windowing.quads().add(
            &layer,
            vxdraw::quads::Quad::new()
                .colors([(255, 0, 0, 127); 4])
                .width(10.0)
                .height(10.0)
                .origin((-5.0, -5.0)),
        );

        let mut dyntex = windowing.dyntex();

        let fireballs = dyntex.add_layer(FIREBALLS, &vxdraw::dyntex::LayerOptions::new());

        let weapons_texture = dyntex.add_layer(WEAPONS, &vxdraw::dyntex::LayerOptions::new());
        let text_layer = windowing.text().add_layer(
            include_bytes!["../../crates/vxdraw/fonts/DejaVuSans.ttf"],
            vxdraw::text::LayerOptions::new(),
        );
        let basic_text = windowing.text().add(
            &text_layer,
            "( ͡° ͜ʖ ͡°)",
            vxdraw::text::TextOptions::new()
                .font_size(40.0)
                .scale(100.0)
                .translation((110.0, 3.2)),
        );

        self.graphics = Some(Graphics {
            basic_text,
            player_quads: vec![handle],
            bullets_texture: fireballs,
            grid: tex,
            weapons_texture,
            windowing,
        });
    }
}

pub fn process_input(s: &mut Input, windowing: &mut VxDraw) {
    for event in windowing.collect_input() {
        if let Event::WindowEvent { event, .. } = event {
            match event {
                WindowEvent::KeyboardInput { input, .. } => {
                    s.register_key(&input);
                }
                WindowEvent::MouseWheel {
                    delta, modifiers, ..
                } => {
                    if let winit::MouseScrollDelta::LineDelta(_, v) = delta {
                        s.register_mouse_wheel(v);
                        if modifiers.ctrl {
                            s.set_ctrl();
                        }
                    }
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    s.register_mouse_input(state, button);
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let pos: (i32, i32) = position.into();
                    s.position_mouse(pos.0, pos.1);
                }
                _ => {}
            }
        }
    }
}

fn move_camera_according_to_input(s: &mut Client) {
    if s.input.is_key_down(Key::D) {
        s.logic.cam.center.x += 5.0;
    }
    if s.input.is_key_down(Key::A) {
        s.logic.cam.center.x -= 5.0;
    }
    if s.input.is_key_down(Key::W) {
        s.logic.cam.center.y -= 5.0;
    }
    if s.input.is_key_down(Key::S) {
        s.logic.cam.center.y += 5.0;
    }
    if s.input.get_ctrl() {
        match s.input.get_mouse_wheel() {
            x if x > 0.0 => {
                if s.logic.cam.zoom < 5.0 {
                    s.logic.cam.zoom *= 1.1;
                }
            }
            x if x < 0.0 => {
                if s.logic.cam.zoom > 0.002 {
                    s.logic.cam.zoom /= 1.1;
                }
            }
            _ => {}
        }
    }

    if s.logic.cam_mode == CameraMode::FollowPlayer {
        if let Some(player) = s.logic.players.get_mut(&s.logic.self_id) {
            s.logic.cam.center -=
                (s.logic.cam.center - player.position - Vec2 { x: 5.0, y: 5.0 }) / 10.0;
        }
    }
}

fn set_gravity(s: &mut Client) {
    if s.input.is_key_toggled_down(Key::G) {
        s.logic.config.gravity_on = !s.logic.config.gravity_on;
        // TODO actually send this to server or something
    }
}
fn toggle_camera_mode(s: &mut Client) {
    if s.input.is_key_toggled_down(Key::F) {
        s.logic.cam_mode = match s.logic.cam_mode {
            CameraMode::FollowPlayer => CameraMode::Interactive,
            CameraMode::Interactive => CameraMode::FollowPlayer,
        };
    }
}
fn update_graphics(s: &mut Client) {
    if let Some(ref mut graphics) = s.graphics {
        let changeset = &s.logic.changed_tiles;
        graphics.windowing.strtex().set_pixels(
            &graphics.grid,
            changeset
                .iter()
                .map(|pos| (pos.0 as u32, pos.1 as u32, Color::Rgba(0, 0, 0, 255))),
        );

        graphics.windowing.dyntex().set_uvs(
            s.logic
                .bullets
                .values()
                .map(|b| (&b.handle, b.current_uv_begin, b.current_uv_end)),
        );

        for b in s.logic.bullets.values() {
            graphics
                .windowing
                .dyntex()
                .set_translation(&b.handle, b.position.into());
        }

        {
            if let Some(player) = s.logic.players.get_mut(&s.logic.self_id) {
                let mouse_in_world = graphics.windowing.to_world_coords(s.input.get_mouse_pos());
                let angle = -(Vec2::from(mouse_in_world) - player.position - PLAYER_CENTER).angle();

                if let Some(ref mut sprite) = player.weapon_sprite {
                    if angle > std::f32::consts::PI / 2.0 || angle < -std::f32::consts::PI / 2.0 {
                        graphics
                            .windowing
                            .dyntex()
                            .set_uv(sprite, (0.0, 1.0), (1.0, 0.0));
                    } else {
                        graphics
                            .windowing
                            .dyntex()
                            .set_uv(sprite, (0.0, 0.0), (1.0, 1.0));
                    }
                    graphics.windowing.dyntex().set_rotation(sprite, Rad(angle));
                }
            }
        }

        upload_player_position(
            &mut s.logic,
            &mut graphics.windowing,
            &graphics.player_quads[0],
        );
    }
    s.logic.changed_tiles.clear();
}
fn draw_graphics(s: &mut Client) {
    if let Some(ref mut graphics) = s.graphics {
        let persp = graphics.windowing.perspective_projection();
        let scale = Matrix4::from_scale(s.logic.cam.zoom);
        let center = s.logic.cam.center;
        // let lookat = Matrix4::look_at(Point3::new(center.x, center.y, -1.0), Point3::new(center.x, center.y, 0.0), Vector3::new(0.0, 0.0, -1.0));
        let trans = Matrix4::from_translation(Vector3::new(-center.x, -center.y, 0.0));
        // info![client.logger, "main", "Okay wth"; "trans" => InDebug(&trans); clone trans];
        graphics.windowing.set_perspective(persp * scale * trans);
        graphics.windowing.draw_frame();
    }
}
fn upload_player_position(
    s: &mut ClientLogic,
    windowing: &mut VxDraw,
    handle: &vxdraw::quads::Handle,
) {
    if let Some(ref mut player) = s.players.get(&s.self_id) {
        if let Some(ref gun_handle) = player.weapon_sprite {
            windowing
                .dyntex()
                .set_translation(gun_handle, (player.position + PLAYER_CENTER).into());
        }
        windowing
            .quads()
            .set_solid_color(handle, Color::Rgba(0, 255, 0, 255));
        windowing
            .quads()
            .set_translation(handle, player.position.into());
    }
}
fn update_bullets_uv(s: &mut ClientLogic) {
    for b in s.bullets.values_mut() {
        let stats = b.get_stats();
        let width_elem = b.animation_sequence % stats.width;
        let height_elem = b.animation_sequence / stats.width;
        let uv_begin = (
            width_elem as f32 / stats.width as f32,
            height_elem as f32 / stats.height as f32,
        );
        let uv_end = (
            (width_elem + 1) as f32 / stats.width as f32,
            (height_elem + 1) as f32 / stats.height as f32,
        );
        b.animation_sequence += 1;
        if b.animation_sequence >= stats.width * stats.height {
            b.animation_sequence = 0;
        }
        let current_uv_begin = (Vec2::from(uv_begin) * Vec2::from(stats.animation_block_end)
            + Vec2::from(stats.animation_block_begin))
        .into();
        let current_uv_end = (Vec2::from(uv_end) * Vec2::from(stats.animation_block_end)).into();
        b.current_uv_begin = current_uv_begin;
        b.current_uv_end = current_uv_end;
    }
}
fn handle_mouse_scroll(_s: &mut Client) {
    // TODO
    /*
    let wheel = s.logic.input.get_mouse_wheel();
    match wheel {
        x if x == 0.0 => {}
        x if x > 0.0 => {
            s.logic.current_weapon = Weapon::Ak47;
            if let Some(this_player) = s.logic.players.get_mut(0) {
                if let Some(ref mut gfx) = s.graphics {
                    let new = gfx.windowing.dyntex().add(
                        &gfx.weapons_texture,
                        dyntex::Sprite::new().width(10.0).height(5.0),
                    );
                    let old = std::mem::replace(&mut this_player.weapon_sprite, Some(new));
                    if let Some(old_id) = old {
                        gfx.windowing.dyntex().remove(old_id);
                    }
                }
            }
        }
        x if x < 0.0 => {
            // s.logic.current_weapon = Weapon::Hellfire;
            // TODO: Switch weapon
        }
        _ => {}
    }
    */
}
