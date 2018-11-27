use glium;
use glium::DisplayBuild;
use glium::glutin;
use glium::glutin::{MouseScrollDelta, VirtualKeyCode as KeyCode};
use graphics::Graphics;
use tilenet::TileNet;
use global::Tile;

use rand;
use input::Input;
use err::*;
use rand::Rng;

use net::{Socket, to_socket_addr};
use net::msg::Message;
use std::net::{SocketAddr};
use specs::{DispatcherBuilder};
use srv::system::MaintainSys;

pub mod game;
pub mod system;
pub mod cam;
use self::game::Game;



pub struct Client {
    game: Game,
    input: Input,
    display: glium::Display,
    graphics: Graphics,

    // Networking
    socket: Socket,
    server: SocketAddr,
}



impl Client {
    pub fn new(server_addr: &str) -> Result<Client, Error> {

        let mut socket = Client::create_socket();
        let server = to_socket_addr(server_addr)?;

        // Init connection
        socket.send_to(Message::Join {snapshot_rate: 20.0}, server)?;
        // Get world metadata
        let (_, msg) = socket.recv().unwrap();
        // TODO reordering will be problematic here, expecting only a certain message
        match msg {
            Message::Welcome {width, height, you, white_base, black_base} => {
                let display = glutin::WindowBuilder::new().build_glium().unwrap();
                let mut game = Game::new(width, height, you, white_base, black_base, display.clone());
                info!("Client received Welcome message");

                let graphics = Graphics::new(display.clone(), &*game.world.read_resource());
                Ok(Client {
                    input: Input::new(),
                    game: game,
                    display: display,
                    graphics: graphics,


                    socket: socket,
                    server: server,
                })
            },
            _ => {
                Err(format_err!("Didn't receive Welcome message (in order...)"))
            },
        }

    }
    pub fn run(&mut self) -> Result<(), Error> {
        self.game.cam.update_win_size(&self.display);

        let mut builder = DispatcherBuilder::new();
        builder.add(MaintainSys, "maintain", &[]);
        let mut dispatcher = builder.build();

        loop {
            // Input
            self.input.update();
            // Handle input events
            for ev in self.display.clone().poll_events() {
                match ev {
                    glutin::Event::Closed => return Ok(()),
                    glutin::Event::MouseMoved(x, y) =>
                        self.input.position_mouse(x, y),
                    glutin::Event::MouseWheel(MouseScrollDelta::LineDelta(_, y), _) =>
                        self.input.register_mouse_wheel(y),
                    glutin::Event::MouseInput(state, button) =>
                        self.input.register_mouse_input(state, button),
                    glutin::Event::KeyboardInput(_, _, _) =>
                        self.input.register_key(ev),
                    glutin::Event::Resized(w, h) => {
                        self.game.cam.width = w;
                        self.game.cam.height = h;
                    }
                    _ => (),
                }
            }
            self.handle_input();

            // Receive messages
            self.socket.update()?;
            let mut messages = Vec::new();
            for msg in self.socket.messages() {
                let msg = msg?;
                messages.push(msg);
            }
            for msg in messages {
                self.handle_message(msg.0, msg.1)?;
            }

            // Update game & send messages
            let packets = self.game.update(&mut dispatcher, &self.input);
            for msg in packets.0 {
                self.socket.send_to(msg, self.server)?;
            }
            for msg_reliable in packets.1 {
                self.socket.send_reliably_to(msg_reliable, self.server, None)?;
            }

            // println!("Transl: {:?}", self.game.get_player_transl());
            // println!("Campos: {:?}", self.game.cam.center);

            // Render
            prof!["Render",
                  self.graphics.render(self.game.cam, &self.game.world)
            ];
            
            // vsync doesn't seem to work on Windows
            // thread::sleep(Duration::from_millis(15));
        }
    }
    fn handle_input(&mut self) {
        // Some interactivity for debugging
        if self.input.key_down(KeyCode::Comma) && self.input.key_toggled(KeyCode::Comma) {
            self.graphics.tilenet_renderer.toggle_smooth();
        }
    }



    /// Currently just ignores unexpected messages
    fn handle_message(&mut self, src: SocketAddr, msg: Message) -> Result<(), Error> {
        if src != self.server {
            bail!("Packet not from server");
        }
        match msg {
            Message::Welcome {width: _, height: _, you: _, white_base: _, black_base: _} => {
            },
            Message::WorldRect {x, y, width, pixels} => {
                let height = pixels.len() / width;
                self.update_tilenet_rect(x, y, width, height, pixels);
            },
            Message::State (snapshot) => {
                self.game.apply_snapshot(snapshot);
            }
            _ => bail!("Wrong message type."),

        };
        Ok(())
    }


    fn update_tilenet_rect(&mut self, x: usize, y: usize, w: usize, h: usize, pixels: Vec<u8>) {
        let tilenet = &mut *self.game.world.write_resource::<TileNet<Tile>>();
        let mut i = 0;
        for y in y..y+h {
            for x in x..x+w {
                tilenet.set(&pixels[i], (x, y));
                i += 1;
            }
        }
        self.graphics.tilenet_renderer.upload_texture(tilenet, x as u32, y as u32, w as u32, h as u32);
    }


    fn create_socket() -> Socket {
        let mut rng = rand::thread_rng();
        loop {
            let p: u16 = 10000 + (rng.gen::<u16>() % 50000);
            let socket = Socket::new(p);
            match socket {
                Ok(socket) => return socket,
                Err(_) => {},
            };
            
        }
    }

}


