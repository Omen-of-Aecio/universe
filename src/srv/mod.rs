use geometry::vec::Vec2;
use net::Socket;
use net::msg::{Message};
use err::*;
use component::*;
use specs;
use tile_net::{TileNet, Collable};
use global::Tile;
use collision::RayCollable;

use num_traits::Float;
use specs::{DispatcherBuilder};
use srv::system::*;

use std::net::SocketAddr;
use std::vec::Vec;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;

pub mod system;
pub mod game;

use self::game::Game;

const WORLD_SIZE: usize = 700;


pub struct Server {
    game: Game,
    connections: HashMap<SocketAddr, specs::Entity>,

    // Networking
    socket: Socket,
}

// Thoughts
// How to store inputs for each player?
// And apply the inputs

impl Server {
    pub fn new() -> Server {
        let size = WORLD_SIZE as f32;
        let mut game = Game::new(WORLD_SIZE, WORLD_SIZE, Vec2::new(size/4.0, size/2.0), Vec2::new(3.0*size/4.0, size/2.0));
        game.generate_world();

        Server {
            game: game,
            connections: HashMap::new(),

            socket: Socket::new(9123).unwrap(),
        }
    }
    pub fn run(&mut self) -> Result<()> {
        let mut dispatcher = DispatcherBuilder::new()
            .add(MoveSys, "move", &[])
            .add(JumpSys, "jump", &[])
            .add(InputSys, "input", &[])
            .build();

        loop {
            // Networking
            self.socket.update()?;

            // Receive messages
            let mut messages = Vec::new();
            for msg in self.socket.messages() {
                let msg = msg.chain_err(|| "Error in received message.")?;
                messages.push(msg);
            }
            for msg in messages {
                self.handle_message(msg.0, msg.1).chain_err(|| "Error in handling message.")?;
            }
            // Send messages
            let message = Message::Players (self.game.get_srv_players());
            Server::broadcast(&mut self.socket, self.connections.keys(), &message).chain_err(|| "Could not broadcast.")?;

            // Logic
            prof!["Logic",
                self.game.update(&mut dispatcher)
            ];
            thread::sleep(Duration::from_millis(16));
        }

    }

    // Made this static to avoid taking &mut self
    // TODO: for consistency, do so with broadcast_reliably as well.
    fn broadcast<'a, I>(socket: &mut Socket, connections: I, msg: &Message) -> Result<()>
        where I: Iterator<Item = &'a SocketAddr>
    {
        for client in connections {
            socket.send_to(msg.clone(), *client)?;
        }
        Ok(())
    }
    fn broadcast_reliably(&mut self, msg: &Message) -> Result<()> {
        for client in self.connections.keys() {
            self.socket.send_reliably_to(msg.clone(), *client)?;
        }
        Ok(())
    }

    fn bullet_fire(&mut self, player_id: u32, direction: Vec2) -> Result<()> {

        let entity = self.game.get_player(player_id);
        let (pos, color) = (
            self.game.world.read::<Pos>(),
            self.game.world.read::<Color>()
        );
        let (pos, color) = (
            pos.get(entity).unwrap(),
            color.get(entity).unwrap(),
            );

        let mut ray = RayCollable::new(pos.transl, direction, *color);

        {
            let tilenet = self.game.world.read_resource::<TileNet<Tile>>();
            ray.solve(&tilenet);
        }
        match ray.hit_tile {
            Some((x, y)) => {
                let x = x as usize;
                let y = y as usize;
                let intensity = 255 - (color.to_intensity() * 255.0) as u8;
                {
                    let mut tilenet = self.game.world.write_resource::<TileNet<Tile>>();
                    tilenet.set_box(&intensity, (x-5, y-5), (x+5, y+5));
                }
                let msg = self.wrap_game_rect(x-5, y-5, 11, 11)?;
                if let Some(msg) = msg {
                    Server::broadcast(&mut self.socket, self.connections.keys(), &msg)?;
                }
            },
            None => {}
        };
        Ok(())
    }

    fn handle_message(&mut self, src: SocketAddr, msg: Message) -> Result<()> {
        // TODO a lot of potential for abstraction/simplification...

        // Will ignore packets from unregistered connections
        match msg {
            Message::Join => self.new_connection(src)?,
            Message::Input (input) => {
                let entity = *self.connections.get(&src).ok_or_else(|| "SocketAddr not registererd as player")?;
                let mut input_resource = self.game.world.write::<PlayerInput>();
                let input_ref = input_resource.get_mut(entity).ok_or_else(|| "Entity doesn't have input")?;
                *input_ref = input;
            },
            Message::ToggleGravity => self.game.toggle_gravity(),
            Message::BulletFire { direction } => {
                let entity = *self.connections.get(&src).ok_or_else(|| "SocketAddr not registererd as player")?;
                let player_id = self.game.world.read::<Player>().get(entity).ok_or_else(|| "Entity not player")?.id;
                self.bullet_fire(player_id, direction)?;
            },
            _ => {}
        }
        Ok(())
    }


    fn new_connection(&mut self, src: SocketAddr) -> Result<()> {
        info!("New connection!");
        // Add new player
        let (w_count, b_count) = self.game.count_player_colors();
        let color = if w_count >= b_count { Color::Black } else { Color::White };
        let player_id = self.game.add_player(color);
        let _ = self.connections.insert(src, self.game.get_player(player_id));
        // Tell about the game size and other meta data
        self.socket.send_to(
            Message::Welcome {
                width: self.game.get_width() as u32,
                height: self.game.get_height() as u32,
                you: player_id,
                players: self.game.get_srv_players(),
                white_base: self.game.white_base,
                black_base: self.game.black_base,
            },
            src).chain_err(|| "Could not send Welcome packet.")?;

        // Send it the whole game
        // We will need to split it up because of limited package size
        let (packet_w, packet_h) = Server::packet_dim(Socket::max_packet_size() as usize);
        let blocks = (self.game.get_width() / packet_w + 1, self.game.get_height() / packet_h + 1);
        for x in 0..blocks.0 {
            for y in 0..blocks.1 {
                let msg = self.wrap_game_rect(x * packet_w, y * packet_w, packet_w, packet_h)?;
                if let Some(msg) = msg {
                    self.socket.send_reliably_to(msg, src)?;
                }
            }
        }
        let srv_player = self.game.get_srv_player(player_id)?;
        self.broadcast_reliably(&Message::NewPlayer (srv_player))
            .chain_err(|| "Could not broadcast_reliably.")?;

        Ok(())
    }

    /// Create message ready for sending
    fn wrap_game_rect(&self, x: usize, y: usize, w: usize, h: usize) -> Result<Option<Message>> {
        let (pixels, w, h) = self.game.get_tilenet_serial_rect(x, y, w, h);
        Ok(Some(Message::WorldRect { x: x, y: y, width: w, pixels: pixels}))
    }

    /// ASSUMPTION: packet size is 2^n
    fn packet_dim(packet_size: usize) -> (usize, usize) {
        let n = (packet_size as f32).log(2.0).floor();
        (2.0.powf((n/2.0).ceil()) as usize, 2.0.powf((n/2.0).floor()) as usize)
    }
}
