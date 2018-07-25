use geometry::vec::Vec2;
use net::Socket;
use net::msg::{Message};
use err::*;
use component::*;
use specs;

use num_traits::Float;
use specs::{DispatcherBuilder};
use srv::system::*;

use std::net::SocketAddr;
use std::vec::Vec;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use std::time::SystemTime;

use conf::Config;

pub mod system;
pub mod game;

use self::game::Game;

#[derive(Clone)]
struct Connection {
    /// Unique id in the ECS
    ecs_id: u32,
    time_since_snapshot: f32,
    snapshot_rate: f32,
}
impl Connection {
    pub fn new(ecs_id: u32, snapshot_rate: f32) -> Connection {
        Connection {
            ecs_id,
            time_since_snapshot: 0.0,
            snapshot_rate,
        }
    }
}

pub struct Server {
    game: Game,
    connections: HashMap<SocketAddr, Connection>,
    socket: Socket,

    /// Frame duration in seconds (used only for how long to sleep. FPS is in GameConfig)
    tick_duration: Duration,
}

impl Server {
    pub fn new(config: Config) -> Server {
        let mut game = Game::new(config.clone(),
                                 Vec2::new((config.world.width/4) as f32, (config.world.height/2) as f32),
                                 Vec2::new((3*config.world.width/4) as f32, (config.world.height/2) as f32));
        game.generate_world();

        Server {
            game: game,
            connections: HashMap::new(),
            socket: Socket::new(9123).unwrap(),

            tick_duration: config.get_srv_tick_duration(),
        }
    }
    pub fn run(&mut self) -> Result<(), Error> {
        let mut builder = DispatcherBuilder::new();
        builder.add(MoveSys, "move", &[]);
        builder.add(JumpSys, "jump", &[]);
        builder.add(InputSys, "input", &[]);
        builder.add(MaintainSys, "maintain", &[]);
        let mut dispatcher = builder.build();

        let mut prev_time = SystemTime::now();
        loop {
            // Networking
            self.socket.update()?;

            // Receive messages
            let mut messages = Vec::new();
            for msg in self.socket.messages() {
                let msg = msg?;
                messages.push(msg);
            }
            for msg in messages {
                self.handle_message(msg.0, msg.1)?;
            }
            // Send messages
            // TODO: delta compression (easy to do)
            let message = Message::State (self.game.create_snapshot());
            Server::broadcast(&mut self.socket, self.connections.keys(), &message)?;

            // Logic
            let now = SystemTime::now();
            let delta_time = now.duration_since(prev_time).expect("duration_since error");
            prof!["Logic",
                self.game.update(&mut dispatcher, ::DeltaTime::from_duration(delta_time))
            ];

            if delta_time < self.tick_duration {
                thread::sleep(self.tick_duration - delta_time);
            }
            prev_time = now;
        }

    }

    // Made this static to avoid taking &mut self
    // TODO: for consistency, do so with broadcast_reliably as well.
    //        .... or make it a member function of Socket? Or map connections?
    fn broadcast<'a, I>(socket: &mut Socket, connections: I, msg: &Message) -> Result<(), Error>
        where I: Iterator<Item = &'a SocketAddr>
    {
        for client in connections {
            socket.send_to(msg.clone(), *client)?;
        }
        Ok(())
    }
    fn broadcast_reliably(&mut self, msg: &Message) -> Result<(), Error> {
        for client in self.connections.keys() {
            self.socket.send_reliably_to(msg.clone(), *client)?;
        }
        Ok(())
    }


    fn handle_message(&mut self, src: SocketAddr, msg: Message) -> Result<(), Error> {
        // TODO a lot of potential for abstraction/simplification...

        // Will ignore packets from unregistered connections
        match msg {
            Message::Join {snapshot_rate} => self.new_connection(src, snapshot_rate)?,
            Message::Input (input) => {
                let con = self.connections.get(&src)
                    .ok_or_else(|| format_err!("SocketAddr not registererd as player"))?;
                self.game.input(con.ecs_id, input)?;
            },
            Message::ToggleGravity => self.game.toggle_gravity(),
            Message::BulletFire { direction } => {
                let con = self.connections.get(&src)
                    .ok_or_else(|| format_err!("SocketAddr not registererd as player"))?;
                self.game.bullet_fire(con.ecs_id, direction)?;
            },
            _ => {}
        }
        Ok(())
    }


    fn new_connection(&mut self, src: SocketAddr, snapshot_rate: f32) -> Result<(), Error> {
        info!("New connection!");
        // Add new player
        let (w_count, b_count) = self.game.count_player_colors();
        let color = if w_count >= b_count { Color::Black } else { Color::White };
        let player_id = self.game.add_player(color);
        let _ = self.connections.insert(src, Connection::new(player_id, snapshot_rate));

        // Tell about the game size and other meta data
        self.socket.send_to(
            Message::Welcome {
                width: self.game.get_width() as u32,
                height: self.game.get_height() as u32,
                you: player_id,
                white_base: self.game.white_base,
                black_base: self.game.black_base,
            },
            src)?;

        // Send it the whole world
        // We will need to split it up because of limited package size
        let (packet_w, packet_h) = Server::packet_dim(Socket::max_payload_size() as usize);
        let blocks = (self.game.get_width() / packet_w + 1, self.game.get_height() / packet_h + 1);
        info!("blocks {:?}", blocks);
        for x in 0..blocks.0 {
            for y in 0..blocks.1 {
                // info!("world packet {},{}", x * packet_w, y * packet_);
                let msg = self.wrap_game_rect(x * packet_w, y * packet_h, packet_w, packet_h);
                if let Some(msg) = msg {
                    self.socket.send_reliably_to(msg, src)?;
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
            Some(Message::WorldRect { x: x, y: y, width: w, pixels: pixels})
        }
    }

    /// ASSUMPTION: packet size is 2^n
    fn packet_dim(mut packet_size: usize) -> (usize, usize) {
        packet_size -= 64; // Assume that the other fields take this amt of bytes...
        let n = (packet_size as f32).log(2.0).floor();
        (2.0.powf((n/2.0).ceil()) as usize, 2.0.powf((n/2.0).floor()) as usize)
    }
}
