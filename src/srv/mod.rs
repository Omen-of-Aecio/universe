use component::*;
use conf::Config;
use err::*;
use geometry::vec::Vec2;
use glocals::*;
use net::Socket;
use net::msg::Message;
use num_traits::Float;
use specs::DispatcherBuilder;
use srv::system::*;
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

impl Server {
    pub fn new(config: &Config) -> Server {
        let mut game = ServerGame::new(
            config,
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
            game,
            connections: HashMap::new(),
            socket: Socket::new(9123).unwrap(),

            tick_duration: config.get_srv_tick_duration(),
        }
    }

    fn handle_message(&mut self, src: SocketAddr, msg: &Message) -> Result<(), Error> {
        // TODO a lot of potential for abstraction/simplification...

        // Will ignore packets from unregistered connections
        match *msg {
            Message::Join { snapshot_rate } => self.new_connection(src, snapshot_rate)?,
            Message::Input(input) => {
                let con = self
                    .connections
                    .get(&src)
                    .ok_or_else(|| format_err!("SocketAddr not registererd as player"))?;
                self.game.input(con.ecs_id, input)?;
            }
            Message::ToggleGravity => self.game.toggle_gravity(),
            Message::BulletFire { direction } => {
                let con = self
                    .connections
                    .get(&src)
                    .ok_or_else(|| format_err!("SocketAddr not registererd as player"))?;
                self.game.bullet_fire(con.ecs_id, direction)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn new_connection(&mut self, src: SocketAddr, snapshot_rate: f32) -> Result<(), Error> {
        info!("New connection!");
        // Add new player
        let (w_count, b_count) = self.game.count_player_colors();
        let color = if w_count >= b_count {
            Color::Black
        } else {
            Color::White
        };
        let player_id = self.game.add_player(color);
        let _ = self
            .connections
            .insert(src, Connection::new(player_id, snapshot_rate));

        // Tell about the game size and other meta data
        self.socket.send_to(
            Message::Welcome {
                width: self.game.get_width() as u32,
                height: self.game.get_height() as u32,
                you: player_id,
                white_base: self.game.white_base,
                black_base: self.game.black_base,
            },
            src,
        )?;

        // Send it the whole world
        // We will need to split it up because of limited package size
        let (packet_w, packet_h) = Server::packet_dim(Socket::max_payload_size() as usize);
        let blocks = (
            self.game.get_width() / packet_w + 1,
            self.game.get_height() / packet_h + 1,
        );
        info!("blocks {:?}", blocks);
        for x in 0..blocks.0 {
            for y in 0..blocks.1 {
                // info!("world packet {},{}, {},{}", x * packet_w, y * packet_h, packet_w, packet_h);
                let msg = self.wrap_game_rect(x * packet_w, y * packet_h, packet_w, packet_h);
                if let Some(msg) = msg {
                    self.socket.send_reliably_to(msg, src, None)?;
                }
            }
        }

        Ok(())
    }

    /// Create message ready for sending
    fn wrap_game_rect(&self, x: usize, y: usize, w: usize, h: usize) -> Option<Message> {
        let (pixels, w, h) = self.game.get_tilenet_serial_rect(x, y, w, h);
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
    let acked_msgs: Rc<Mutex<VecDeque<(SocketAddr, u32)>>> =
        Rc::new(Mutex::new(VecDeque::new()));
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
            s.handle_message(msg.0, &msg.1)?;
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
            let message = Message::State(s.game.create_snapshot(con.last_snapshot));
            // debug!("Snapshot"; "size" => bincode::serialized_size(&message).unwrap(),
            // "last snapshot" => con.last_snapshot);
            let dest = *dest;
            let current_frame = s.game.frame_nr();
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
            s.game
                .update(&mut dispatcher, ::DeltaTime::from_duration(delta_time))
        ];

        if delta_time < s.tick_duration {
            thread::sleep(s.tick_duration - delta_time);
        }
        prev_time = now;
    }
}
