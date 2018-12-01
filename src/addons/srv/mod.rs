use self::system::*;
use glocals::component::*;
use glocals::conf::Config;
use glocals::*;
use libs::geometry::vec::Vec2;
use libs::net::msg::Message;
use libs::net::Socket;
use num_traits::Float;
use specs::DispatcherBuilder;
use std::{
    collections::{HashMap, VecDeque},
    net::SocketAddr,
    rc::Rc,
    sync::Mutex,
    thread,
    time::SystemTime,
    vec::Vec,
};

pub mod diff;
pub mod game;
pub mod system;

use self::game::generate_world;

impl Connection {
    pub fn new(ecs_id: u32, snapshot_rate: f32) -> Connection {
        Connection {
            ecs_id,
            last_snapshot: 0,
            snapshot_rate,
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct DeltaTime {
    secs: f32,
}

impl DeltaTime {
    pub fn from_duration(duration: std::time::Duration) -> DeltaTime {
        DeltaTime {
            secs: duration.as_secs() as f32 + (duration.subsec_nanos() as f32) / 1_000_000_000.0,
        }
    }
}

pub fn create_server<'a>(s: Main<'a>) -> Server<'a> {
    let config = s.config.clone().unwrap();
    let mut game = game::create_servergame(
        &config,
        Vec2::new(
            (config.world.width / 4) as f32,
            (config.world.height / 2) as f32,
        ),
        Vec2::new(
            (3 * config.world.width / 4) as f32,
            (config.world.height / 2) as f32,
        ),
    );
    generate_world(&mut game);

    Server {
        main: s,
        game,
        connections: HashMap::new(),
        socket: Socket::new(9123).unwrap(),

        tick_duration: config.get_srv_tick_duration(),
    }
}

fn handle_message(s: &mut Server, src: SocketAddr, msg: &Message) -> Result<(), Error> {
    // TODO a lot of potential for abstraction/simplification...

    // Will ignore packets from unregistered connections
    match *msg {
        Message::Join { snapshot_rate } => new_connection(s, src, snapshot_rate)?,
        Message::Input(input) => {
            let con = s
                .connections
                .get(&src)
                .ok_or_else(|| format_err!("SocketAddr not registererd as player"))?;
            game::input(&mut s.game, con.ecs_id, input)?;
        }
        Message::ToggleGravity => game::toggle_gravity(&mut s.game),
        Message::BulletFire { direction } => {
            let con = s
                .connections
                .get(&src)
                .ok_or_else(|| format_err!("SocketAddr not registererd as player"))?;
            game::bullet_fire(&mut s.game, con.ecs_id, direction)?;
        }
        _ => {}
    }
    Ok(())
}

fn new_connection(s: &mut Server, src: SocketAddr, snapshot_rate: f32) -> Result<(), Error> {
    info!("New connection!");
    // Add new player
    let (w_count, b_count) = game::count_player_colors(&s.game);
    let color = if w_count >= b_count {
        Color::Black
    } else {
        Color::White
    };
    let player_id = game::add_player(&mut s.game, color);
    let _ = s
        .connections
        .insert(src, Connection::new(player_id, snapshot_rate));

    // Tell about the game size and other meta data
    s.socket.send_to(
        Message::Welcome {
            width: game::get_width(&s.game) as u32,
            height: game::get_height(&s.game) as u32,
            you: player_id,
            white_base: s.game.white_base,
            black_base: s.game.black_base,
        },
        src,
    )?;

    // Send it the whole world
    // We will need to split it up because of limited package size
    let (packet_w, packet_h) = packet_dim(Socket::max_payload_size() as usize);
    let blocks = (
        game::get_width(&s.game) / packet_w + 1,
        game::get_height(&s.game) / packet_h + 1,
    );
    info!("blocks {:?}", blocks);
    for x in 0..blocks.0 {
        for y in 0..blocks.1 {
            // info!("world packet {},{}, {},{}", x * packet_w, y * packet_h, packet_w, packet_h);
            let msg = wrap_game_rect(s, x * packet_w, y * packet_h, packet_w, packet_h);
            if let Some(msg) = msg {
                s.socket.send_reliably_to(msg, src, None)?;
            }
        }
    }

    Ok(())
}

/// Create message ready for sending
fn wrap_game_rect(s: &Server, x: usize, y: usize, w: usize, h: usize) -> Option<Message> {
    let (pixels, w, h) = game::get_tilenet_serial_rect(&s.game, x, y, w, h);
    if w * h == 0 {
        warn!("zero-size chunk of the world requested");
        None
    } else {
        Some(Message::WorldRect {
            x,
            y,
            width: w,
            pixels,
        })
    }
}

/// ASSUMPTION: packet size is 2^n
fn packet_dim(mut packet_size: usize) -> (usize, usize) {
    packet_size -= 64; // Assume that the other fields take this amt of bytes...
    let n = (packet_size as f32).log(2.0).floor();
    (
        2.0.powf((n / 2.0).ceil()) as usize,
        2.0.powf((n / 2.0).floor()) as usize,
    )
}

pub fn run(s: &mut Server) -> Result<(), Error> {
    let mut builder = DispatcherBuilder::new();
    builder.add(MoveSys, "move", &[]);
    builder.add(JumpSys, "jump", &[]);
    builder.add(InputSys, "input", &[]);
    builder.add(MaintainSys, "maintain", &[]);
    builder.add(DiffSys, "diff", &[]);
    let mut dispatcher = builder.build();

    let mut prev_time = SystemTime::now();

    // Used to store a 'queue' of snapshots that got ACK'd
    let acked_msgs: Rc<Mutex<VecDeque<(SocketAddr, u32)>>> = Rc::new(Mutex::new(VecDeque::new()));
    loop {
        // Networking
        s.socket.update()?;

        // Receive messages
        let mut messages = Vec::new();
        for msg in s.socket.messages() {
            let msg = msg?;
            messages.push(msg);
        }
        for msg in messages {
            handle_message(s, msg.0, &msg.1)?;
        }

        // Check ACKs
        {
            let mut acked_msgs = acked_msgs.lock().unwrap();
            for (dest, frame) in acked_msgs.iter() {
                if let Some(con) = s.connections.get_mut(&dest) {
                    if con.last_snapshot < *frame {
                        con.last_snapshot = *frame;
                    }
                }
            }
            *acked_msgs = VecDeque::new();
        }
        // Send messages
        for (dest, con) in s.connections.iter() {
            let message = Message::State(game::create_snapshot(&s.game, con.last_snapshot));
            // debug!("Snapshot"; "size" => bincode::serialized_size(&message).unwrap(),
            // "last snapshot" => con.last_snapshot);
            let dest = *dest;
            let current_frame = game::frame_nr(&s.game);
            let acked_msgs = acked_msgs.clone();
            s.socket.send_reliably_to(
                message,
                dest,
                // (WONDERING: a bit confused how I this closure ends up being 'static - what if the
                // Box dies?)
                Some(Box::new(move || {
                    acked_msgs.lock().unwrap().push_back((dest, current_frame));
                })),
            )?;
        }

        // Logic
        let now = SystemTime::now();
        let delta_time = now.duration_since(prev_time).expect("duration_since error");
        prof![
            "Logic",
            game::update(
                &mut s.game,
                &mut dispatcher,
                DeltaTime::from_duration(delta_time)
            )
        ];

        if delta_time < s.tick_duration {
            thread::sleep(s.tick_duration - delta_time);
        }
        prev_time = now;
    }
}
