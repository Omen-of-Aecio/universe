use super::graphics::Graphics;
use addons::srv::{
    diff::{Entity, Snapshot},
    system::MaintainSys,
};
use glium::glutin::{MouseScrollDelta, VirtualKeyCode as KeyCode};
use glium::{self, glutin, Display, DisplayBuild};
use glocals::component::*;
use glocals::{game::CameraMode, game::Game, input::Input, Tile, *};
use libs::geometry::cam::Camera;
use libs::geometry::vec::Vec2;
use libs::net::msg::Message;
use libs::net::{to_socket_addr, Socket};
use rand;
use rand::Rng;
use specs;
use specs::DispatcherBuilder;
use specs::{Dispatcher, Join, LazyUpdate, World};
use std::cmp::min;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::vec::Vec;
use tilenet::TileNet;

/// Puts entity mapping into the HashMap resource. The HashMap is maintained every frame so
/// this only needs to be done when it otherwise poses a problem that the hashmap is not
/// immediately updated.
pub fn register_entity(s: &mut Game, id: u32, ent: specs::Entity) {
    s.world
        .write_resource::<HashMap<u32, specs::Entity>>()
        .insert(id, ent);
}
pub fn apply_snapshot(s: &mut Game, snapshot: Snapshot) {
    let mut added_entities: Vec<(u32, specs::Entity)> = Vec::new();
    {
        let updater = s.world.read_resource::<LazyUpdate>();
        for (id, entity) in snapshot.entities.into_iter() {
            match entity {
                Some(Entity { components }) => {
                    match get_entity(s, id) {
                        Some(this_ent) => {
                            components.modify_existing(&*updater, this_ent);
                        }
                        None => {
                            // TODO: maybe need to care about type (Player/Bullet)
                            let ent = components.insert(&*updater, &*s.world.entities(), id);
                            added_entities.push((id, ent));
                        }
                    }
                }
                // This means the entity was deleted
                None => match get_entity(s, id) {
                    Some(this_ent) => {
                        s.world.entities().delete(this_ent).unwrap();
                    }
                    None => error!("Server removed entity not owned by me"),
                },
            }
        }
    }
    for (id, ent) in added_entities {
        register_entity(s, id, ent);
    }
}

pub fn print(s: &Game) {
    // info!("TileNet"; "content" => format!["{:?}", self.get_tilenet()]);
}
pub fn get_player_transl(s: &Game) -> Option<Vec2> {
    let pos = s.world.read_storage::<Pos>();
    get_you(s).and_then(|you| pos.get(you).map(|pos| pos.transl))
}
pub fn get_you(s: &Game) -> Option<specs::Entity> {
    get_entity(s, s.you)
}
pub fn get_entity(s: &Game, id: u32) -> Option<specs::Entity> {
    s.world
        .read_resource::<HashMap<u32, specs::Entity>>()
        .get(&id)
        .cloned()
}
// Access //
pub fn get_tilenet_serial_rect(s: &Game, x: usize, y: usize, w: usize, h: usize) -> Vec<Tile> {
    let tilenet = &*s.world.read_resource::<TileNet<Tile>>();
    let w = min(x + w, tilenet.get_size().0) as isize - x as isize;
    let h = min(y + h, tilenet.get_size().1) as isize - y as isize;
    if w == 0 || h == 0 {
        return Vec::new();
    }
    let w = w as usize;
    let h = h as usize;

    let pixels: Vec<u8> = tilenet
        .view_box((x, x + w, y, y + h))
        .map(|x| *x.0)
        .collect();
    assert!(pixels.len() == w * h);
    pixels
}

/// Returns (white count, black count)
pub fn count_player_colors(s: &Game) -> (u32, u32) {
    let mut count = (0, 0);
    let (player, color) = {
        (
            s.world.read_storage::<Player>(),
            s.world.read_storage::<Color>(),
        )
    };
    for (_, color) in (&player, &color).join() {
        match *color {
            Color::Black => count.0 += 1,
            Color::White => count.1 += 1,
        }
    }
    count
}

/// Returns (messages to send, messages to send reliably)
fn handle_game_input(s: &mut Game, input: &Input) -> (Vec<Message>, Vec<Message>) {
    let mut msg = Vec::new();
    let mut msg_reliable = Vec::new();
    if input.key_toggled_down(KeyCode::G) {
        msg.push(Message::ToggleGravity)
    }

    // Zooming..
    if input.key_down(KeyCode::N) {
        s.cam.zoom += 0.1;
    }
    if input.key_down(KeyCode::E) {
        s.cam.zoom -= 0.1;
    }

    // Mouse
    if input.mouse() {
        // Update camera offset //
        if let CameraMode::Interactive = s.cam_mode {
            let mut offset = input.mouse_moved() / s.cam.zoom;
            offset.x = -offset.x;
            s.cam.center += offset;
        }

        // Fire weapon //
        if let Some(transl) = get_player_transl(s) {
            let mouse_world_pos = s.cam.screen_to_world(input.mouse_pos());
            let dir = mouse_world_pos - transl;
            let msg = Message::BulletFire { direction: dir };
            msg_reliable.push(msg);
        }
    }

    // Zooming
    const ZOOM_FACTOR: f32 = 1.2;
    let y = input.mouse_wheel();
    if y > 0.0 {
        s.cam.zoom *= f32::powf(ZOOM_FACTOR, y as f32);
    } else if y < 0.0 {
        s.cam.zoom /= f32::powf(ZOOM_FACTOR, -y as f32);
    }

    msg_reliable.push(Message::Input(input.create_player_input()));
    (msg, msg_reliable)
}

/// Returns (messages to send, messages to send reliably)
pub fn update(
    s: &mut Game,
    dispatcher: &mut Dispatcher,
    input: &Input,
) -> (Vec<Message>, Vec<Message>) {
    s.world.maintain();
    // ^^ XXX maintain before rest, because previously in this frame we handled input & network pacakets
    s.vectors.clear(); // clear debug geometry
    let ret = handle_game_input(s, input);
    if let (CameraMode::FollowPlayer, Some(transl)) = (s.cam_mode, get_player_transl(s)) {
        s.cam.center = transl;
    }
    *s.world.write_resource() = s.cam;
    dispatcher.dispatch(&s.world.res);
    ret
}

pub fn create_client(server_addr: &str) -> Result<Client, Error> {
    let mut socket = create_socket();
    let server = to_socket_addr(server_addr)?;

    // Init connection
    socket.send_to(
        Message::Join {
            snapshot_rate: 20.0,
        },
        server,
    )?;
    // Get world metadata
    let (_, msg) = socket.recv().unwrap();
    // TODO reordering will be problematic here, expecting only a certain message
    match msg {
        Message::Welcome {
            width,
            height,
            you,
            white_base,
            black_base,
        } => {
            let display = glutin::WindowBuilder::new().build_glium().unwrap();
            let mut game = create_game(width, height, you, white_base, black_base, &display);
            info!("Client received Welcome message");

            let graphics = Graphics::new(&display, &*game.world.read_resource());
            Ok(Client {
                input: Input::new(),
                game,
                display,
                graphics,

                socket,
                server,
            })
        }
        _ => Err(format_err!("Didn't receive Welcome message (in order...)")),
    }
}

pub fn update_win_size(camera: &mut Camera, display: &Display) {
    match display.get_window() {
        Some(x) => match x.get_inner_size() {
            Some((x, y)) => {
                camera.update_win_size((x, y));
            }
            None => {}
        },
        None => {}
    }
}

pub fn create_game(
    width: u32,
    height: u32,
    you: u32,
    white_base: Vec2,
    black_base: Vec2,
    display: &glium::Display,
) -> Game {
    let mut cam = Camera::default();
    update_win_size(&mut cam, display);

    let world = {
        let mut w = World::new();
        // All components types should be registered before working with them
        w.register_with_storage::<_, Pos>(ComponentStorage::normal);
        w.register_with_storage::<_, Vel>(ComponentStorage::normal);
        w.register_with_storage::<_, Force>(ComponentStorage::normal);
        w.register_with_storage::<_, Jump>(ComponentStorage::normal);
        w.register_with_storage::<_, Shape>(ComponentStorage::normal);
        w.register_with_storage::<_, Color>(ComponentStorage::normal);
        w.register_with_storage::<_, Player>(ComponentStorage::normal);
        w.register_with_storage::<_, UniqueId>(ComponentStorage::normal);

        // The ECS system owns the TileNet
        let mut tilenet = TileNet::<Tile>::new(width as usize, height as usize);

        w.add_resource(tilenet);
        w.add_resource(cam);
        w.add_resource(HashMap::<u32, specs::Entity>::new());

        w
    };

    Game {
        world,
        cam,
        you,
        white_base,
        black_base,
        vectors: Vec::new(),
        cam_mode: CameraMode::FollowPlayer,
    }
}

pub fn run(client: &mut Client) -> Result<(), Error> {
    update_win_size(&mut client.game.cam, &client.display);

    let mut builder = DispatcherBuilder::new();
    builder.add(MaintainSys, "maintain", &[]);
    let mut dispatcher = builder.build();

    loop {
        // Input
        client.input.update();
        // Handle input events
        for ev in client.display.clone().poll_events() {
            match ev {
                glutin::Event::Closed => return Ok(()),
                glutin::Event::MouseMoved(x, y) => client.input.position_mouse(x, y),
                glutin::Event::MouseWheel(MouseScrollDelta::LineDelta(_, y), _) => {
                    client.input.register_mouse_wheel(y)
                }
                glutin::Event::MouseInput(state, button) => {
                    client.input.register_mouse_input(state, button)
                }
                glutin::Event::KeyboardInput(_, _, _) => client.input.register_key(&ev),
                glutin::Event::Resized(w, h) => {
                    client.game.cam.width = w;
                    client.game.cam.height = h;
                }
                _ => (),
            }
        }
        handle_input(client);

        // Receive messages
        client.socket.update()?;
        let mut messages = Vec::new();
        for msg in client.socket.messages() {
            let msg = msg?;
            messages.push(msg);
        }
        for msg in messages {
            handle_message(client, msg.0, msg.1)?;
        }

        // Update game & send messages
        let packets = update(&mut client.game, &mut dispatcher, &client.input);
        for msg in packets.0 {
            client.socket.send_to(msg, client.server)?;
        }
        for msg_reliable in packets.1 {
            client
                .socket
                .send_reliably_to(msg_reliable, client.server, None)?;
        }

        // println!("Transl: {:?}", client.game.get_player_transl());
        // println!("Campos: {:?}", client.game.cam.center);

        // Render
        prof![
            "Render",
            client.graphics.render(client.game.cam, &client.game.world)
        ];

        // vsync doesn't seem to work on Windows
        // thread::sleep(Duration::from_millis(15));
    }
}

fn handle_input(s: &mut Client) {
    // Some interactivity for debugging
    if s.input.key_down(KeyCode::Comma) && s.input.key_toggled(KeyCode::Comma) {
        s.graphics.tilenet_renderer.toggle_smooth();
    }
}

/// Currently just ignores unexpected messages
fn handle_message(s: &mut Client, src: SocketAddr, msg: Message) -> Result<(), Error> {
    if src != s.server {
        bail!("Packet not from server");
    }
    match msg {
        Message::Welcome { .. } => {}
        Message::WorldRect {
            x,
            y,
            width,
            pixels,
        } => {
            let height = pixels.len() / width;
            update_tilenet_rect(s, x, y, width, height, &pixels);
        }
        Message::State(snapshot) => {
            apply_snapshot(&mut s.game, snapshot);
        }
        _ => bail!("Wrong message type."),
    };
    Ok(())
}

fn update_tilenet_rect(s: &mut Client, x: usize, y: usize, w: usize, h: usize, pixels: &[u8]) {
    let tilenet = &mut *s.game.world.write_resource::<TileNet<Tile>>();
    let mut count = 0;
    for y in y..y + h {
        for x in x..x + w {
            tilenet.set(&pixels[count], (x, y));
            count += 1;
        }
    }
    s.graphics
        .tilenet_renderer
        .upload_texture(tilenet, x as u32, y as u32, w as u32, h as u32);
}

fn create_socket() -> Socket {
    let mut rng = rand::thread_rng();
    loop {
        let p: u16 = 10000 + (rng.gen::<u16>() % 50000);
        let socket = Socket::new(p);
        if let Ok(socket) = socket {
            return socket;
        }
    }
}
