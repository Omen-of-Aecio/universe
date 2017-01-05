use geometry::vec::Vec2;
use world;
use world::World;
use net::{Message, Socket};
use err::Result;
use std::cmp::min;
use std::thread;
use std::time::Duration;
use world::color::Color;
use world::player::Player;

use num_traits::Float;

use std::net::SocketAddr;
use std::vec::Vec;

const WORLD_SIZE: usize = 700;

pub struct Server {
    world: World,

    // Networking
    socket: Socket,
    connections: Vec<SocketAddr>,
}

impl Server {
    pub fn new() -> Server {
        let pos = Vec2::new(WORLD_SIZE as f32 / 2.0, WORLD_SIZE as f32/2.0);
        let size = WORLD_SIZE as f32;
        let mut world = World::new(WORLD_SIZE, WORLD_SIZE, Vec2::new(size/4.0, size/2.0), Vec2::new(3.0*size/4.0, size/2.0));

        Server {
            world: world,

            socket: Socket::new(9123).unwrap(),
            connections: Vec::new(),
        }
    }
    pub fn run(&mut self) -> Result<()> {
        loop {
            for msg in &mut self.socket.messages().unwrap() {
                match msg {
                    Ok((src, msg)) => {
                        self.handle_message(src, msg);
                    },
                    Err(e) => return Err(e),
                }
            }


            // Logic
            // prof!["Logic", self.world.update(&self.input)];
            thread::sleep(Duration::from_millis(30));
        }

    }

    fn handle_message(&mut self, src: SocketAddr, msg: Message) -> Result<()> {
        match msg {
            Message::Join => self.new_connection(src)?,
            _ => {}
        }
        Ok(())
    }

    fn new_connection(&mut self, src: SocketAddr) -> Result<()> {
        self.connections.push(src);
        // Add new player
        self.world.add_new_player(Color::Black);
        // Tell about the world size and other meta data
        self.socket.send_to(
            Message::Welcome {
                width: self.world.get_width(),
                height: self.world.get_height(),
                you_index: self.world.players.len() - 1,
                players: self.world.players.iter().map(|x| x.shape.color).collect(),
                white_base: self.world.white_base,
                black_base: self.world.black_base,
            },
            src);

        // Send it the whole world
        // We will need to split it up because of limited package size
        let dim = Server::packet_dim(Socket::max_packet_size());
        let blocks = (self.world.get_width() / dim.0 + 1, self.world.get_height() / dim.1 + 1);
        println!("NUM BLOCKS = {}, {}", blocks.0, blocks.1);
        for x in 0..blocks.0 {
            for y in 0..blocks.1 {
                self.send_world_rect(x * dim.0, y * dim.0, dim.0, dim.1, src)?;
                thread::sleep(Duration::from_millis(5));
            }
        }

        Ok(())
    }

    fn send_world_rect(&mut self, x: usize, y: usize, w: usize, h: usize, dest: SocketAddr) -> Result<()> {
        let w = min(x + w, self.world.tilenet.get_size().0) - x;
        let h = min(y + h, self.world.tilenet.get_size().1) - y;

        println!("Server World Rect: {}, {}; {}, {}", x, y, w, h);
        let pixels: Vec<u8> = self.world.tilenet.view_box((x, x+w, y, y+h)).map(|x| *x.0).collect();
        assert!(pixels.len() == w*h);
        let msg = Message::WorldRect { x: x, y: y, width: w, height: h, pixels: pixels};
        self.socket.send_to(msg, dest);
        Ok(())
    }

    /// ASSUMPTION: packet size is 2^n
    fn packet_dim(packet_size: usize) -> (usize, usize) {
        let n = (packet_size as f32).log(2.0).floor();
        (2.0.powf((n/2.0).ceil()) as usize, 2.0.powf((n/2.0).floor()) as usize)
    }
}

