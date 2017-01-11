use geometry::vec::Vec2;
use world;
use world::World;
use net::Socket;
use net::msg::Message;
use world::color::Color;
use world::player::Player;
use glium::glutin::VirtualKeyCode as KeyCode;
use input::PlayerInput;
use err::{Error, Result};

use num_traits::Float;

use std::net::SocketAddr;
use std::vec::Vec;
use std::collections::HashMap;
use std::cmp::min;
use std::thread;
use std::time::Duration;

const WORLD_SIZE: usize = 700;

const ACCELERATION: f32 = 0.35;

pub struct Server {
    world: World,
    players: HashMap<SocketAddr, PlayerData>,

    // Networking
    socket: Socket,
}

// Thoughts
// How to store inputs for each player?
// And apply the inputs

impl Server {
    pub fn new() -> Server {
        let pos = Vec2::new(WORLD_SIZE as f32 / 2.0, WORLD_SIZE as f32/2.0);
        let size = WORLD_SIZE as f32;
        let mut world = World::new(WORLD_SIZE, WORLD_SIZE, Vec2::new(size/4.0, size/2.0), Vec2::new(3.0*size/4.0, size/2.0), true);

        Server {
            world: world,
            players: HashMap::new(),

            socket: Socket::new(9123).unwrap(),
        }
    }
    pub fn run(&mut self) -> Result<()> {
        loop {
            let messages: Vec<Result<(SocketAddr, Message)>> = self.socket.messages().collect();
            for msg in messages {
                match msg {
                    Ok((src, msg)) => {
                        self.handle_message(src, msg)?;
                    },
                    Err(e) => return Err(e),
                }
            }
            // TODO: Unnecessary clone?
            let players = self.players.clone();

            // Handle input
            for player in players.values() {
                self.handle_input(player.input, player.nr);
            }

            // TODO: Logic and networking shouldn't necessary be done equally frequently.

            // Networking
            self.socket.update()?;
            let message = Message::PlayerPos (players.values().map(|p| self.world.players[p.nr].shape.pos).collect());
            self.broadcast(&message);

            // Logic
            prof!["Logic", self.world.update()];
            thread::sleep(Duration::from_millis(16));
        }

    }

    fn broadcast(&mut self, msg: &Message) {
        for client in self.players.keys() {
            self.socket.send_to(msg.clone(), *client);
        }
    }

    fn handle_message(&mut self, src: SocketAddr, msg: Message) -> Result<()> {
        match msg {
            Message::Join => self.new_connection(src)?,
            Message::Input (input) => {
                match self.players.get_mut(&src) {
                    Some(ref mut player_data) => player_data.input = input,
                    None => bail!("Received 'Input' messages from player with unregistered connection."),
                }
            },
            Message::ToggleGravity => self.world.gravity_on = !self.world.gravity_on,
            _ => {}
        }
        Ok(())
    }

    fn handle_input(&mut self, input: PlayerInput, player_nr: usize) {
        if input.left {
            self.world.players[player_nr].accelerate(Vec2::new(-ACCELERATION, 0.0));
        }
        if input.right {
            self.world.players[player_nr].accelerate(Vec2::new(ACCELERATION, 0.0));

        }
        if input.up {
            if self.world.gravity_on {
                self.world.players[player_nr].jump();
            } else {
                self.world.players[player_nr].accelerate(Vec2::new(0.0, ACCELERATION));
            }
        }
        if input.down {
            if !self.world.gravity_on {
                self.world.players[player_nr].accelerate(Vec2::new(0.0, -ACCELERATION));
            }
        }
        /*
        if input.key_toggled_down(KeyCode::G) {
            self.gravity_on = ! self.gravity_on;
        }
        */
    }

    fn new_connection(&mut self, src: SocketAddr) -> Result<()> {
        info!("New connection!");
        // Add new player
        let player_nr = self.world.add_new_player(Color::Black);
        let _ = self.players.insert(src, PlayerData::new(player_nr));
        // Tell about the world size and other meta data
        self.socket.send_to(
            Message::Welcome {
                width: self.world.get_width(),
                height: self.world.get_height(),
                you_index: player_nr,
                players: self.world.players.iter().map(|x| x.shape.color).collect(),
                white_base: self.world.white_base,
                black_base: self.world.black_base,
            },
            src);

        // Send it the whole world
        // We will need to split it up because of limited package size
        let dim = Server::packet_dim(Socket::max_packet_size());
        let blocks = (self.world.get_width() / dim.0 + 1, self.world.get_height() / dim.1 + 1);
        for x in 0..blocks.0 {
            for y in 0..blocks.1 {
                self.send_world_rect(x * dim.0, y * dim.0, dim.0, dim.1, src)?;
                // thread::sleep(Duration::from_millis(15));
            }
        }

        Ok(())
    }

    fn send_world_rect(&mut self, x: usize, y: usize, w: usize, h: usize, dest: SocketAddr) -> Result<()> {
        let w = min(x + w, self.world.tilenet.get_size().0) - x;
        let h = min(y + h, self.world.tilenet.get_size().1) - y;

        let pixels: Vec<u8> = self.world.tilenet.view_box((x, x+w, y, y+h)).map(|x| *x.0).collect();
        assert!(pixels.len() == w*h);
        let msg = Message::WorldRect { x: x, y: y, width: w, height: h, pixels: pixels};
        self.socket.send_reliably_to(msg, dest)?;
        Ok(())
    }

    /// ASSUMPTION: packet size is 2^n
    fn packet_dim(packet_size: usize) -> (usize, usize) {
        let n = (packet_size as f32).log(2.0).floor();
        (2.0.powf((n/2.0).ceil()) as usize, 2.0.powf((n/2.0).floor()) as usize)
    }
}

#[derive(Copy, Clone)]
struct PlayerData {
    input: PlayerInput,
    nr: usize,
}
impl PlayerData {
    pub fn new(nr: usize) -> PlayerData {
        PlayerData {
            input: PlayerInput::default(),
            nr: nr,
        }
    }
}
