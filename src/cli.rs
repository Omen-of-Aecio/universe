use geometry::vec::Vec2;
use glium;
use glium::{DisplayBuild, glutin};
use glium::glutin::{ElementState, MouseButton, MouseScrollDelta, VirtualKeyCode as KeyCode};
use input::Input;
use world::World;
use world::player::Player;
use graphics::{Graphics, screen_to_world};
use err::*;
use rand;
use rand::Rng;

use net::Socket;
use net::msg::Message;
use std::net::{SocketAddr, SocketAddrV4, Ipv4Addr};
use std::iter::Iterator;

/* Should go, together with some logic, to some camera module (?) */
enum CameraMode {
    Interactive,
    FollowPlayer,
}

pub struct Client {
    display: glium::Display,
    window_size: (u32, u32),
    input: Input,
    graphics: Graphics,
    world: World,

    player_nr: usize,

    // Camera & input (for now)
    cam_mode: CameraMode,
    //   following is used only if INTERACTIVE camera mode
    zoom: f32,
    center: Vec2,

    // Networking
    socket: Socket,
    server: SocketAddr,
}



impl Client {
    pub fn new(server_addr: &str) -> Result<Client> {

        let mut socket = Client::create_socket();
        let server = to_socket_addr(server_addr)?;

        // Init connection
        socket.send_to(Message::Join, server).chain_err(|| "Cannot send Join message.")?;
        // Get world metadata
        let (_, msg) = socket.recv().unwrap();
        // TODO reordering will be problematic here, expecting only a certain message
        match msg {
            Message::Welcome {width, height, you_index, players, white_base, black_base} => {
                let mut world = World::new(width, height, white_base, black_base, false);

                info!("Client create new world");
                for color in players {
                    world.add_new_player(color);
                    info!("Client add new player");
                }

                let display = glutin::WindowBuilder::new().build_glium().unwrap();
                let graphics = Graphics::new(display.clone(), &world);
                Ok(Client {
                    display: display,
                    window_size: (0, 0),
                    input: Input::new(),
                    graphics: graphics,
                    world: world,
                    player_nr: you_index,

                    cam_mode: CameraMode::FollowPlayer,
                    zoom: 1.0,
                    center: Vec2::new(0.0, 0.0),

                    socket: socket,
                    server: server,
                })
            },
            _ => {
                Err("Didn't receive Welcome message (in order...)".into())
            },
        }

    }
    pub fn run(&mut self) -> Result<()> {
        self.window_size = self.display.get_window().unwrap().get_inner_size().unwrap();
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
                    glutin::Event::Resized(w, h) =>
                        self.window_size = (w, h),
                    _ => (),
                }
            }
            self.handle_input()?;
            self.send_input()?;

            // Networking
            self.socket.update()?;
            let mut messages = Vec::new();
            for msg in self.socket.messages() {
                let msg = msg.chain_err(|| "Client: error in received message.")?;
                messages.push(msg);
            }
            for msg in messages {
                self.handle_message(msg.0, msg.1).chain_err(|| "Client: error in handling message.")?;
            }

            // Some interactivity for debugging
            if self.input.key_down(glutin::VirtualKeyCode::Comma) && self.input.key_toggled(glutin::VirtualKeyCode::Comma) {
                self.graphics.tilenet_renderer.toggle_smooth();
            }
            // Zooming..
            if self.input.key_down(glutin::VirtualKeyCode::N) {
                self.zoom += 0.1;
            }
            if self.input.key_down(glutin::VirtualKeyCode::E) {
                self.zoom -= 0.1;
            }

            // Render
            let cam_pos = self.get_cam_pos();
            prof!["Render",
                  self.graphics.render(cam_pos,
                                       self.zoom,
                                       self.window_size.0,
                                       self.window_size.1,
                                       &self.world)];
            
            // vsync doesn't seem to work on Windows
            // thread::sleep(Duration::from_millis(15));
        }
    }

    fn get_cam_pos(&self) -> Vec2 {
        match self.cam_mode {
            CameraMode::Interactive => self.center,
            CameraMode::FollowPlayer => self.world.players[self.player_nr].shape.pos,
        }
    }

    fn send_input(&mut self) -> Result<()> {
        let msg = Message::Input (self.input.create_player_input());
        self.socket.send_reliably_to(msg, self.server)?;
        Ok(())
    }

    /// Currently just ignores unexpected messages
    fn handle_message(&mut self, src: SocketAddr, msg: Message) -> Result<()> {
        if src != self.server {
            bail!("Packet not from server");
        }
        match msg {
            Message::Welcome {width: _, height: _, you_index: _, players: _, white_base: _, black_base: _} => {
            },
            Message::WorldRect {x, y, width, pixels} => {
                let height = pixels.len() / width;
                self.receive_world(x, y, width, height, pixels);
            },
            Message::PlayerPos (pos) => {
                for (i, pos) in pos.iter().enumerate() {
                    if i < self.world.players.len() {
                        self.world.players[i].shape.pos = *pos;
                    } else {
                        warn!("Received position on a player which I have not registered (out of bounds).");
                    }
                }
            },
            Message::NewPlayer {nr, color} => {
                info!("New player has joined"; "nr" => nr);
                let nr = nr as usize;
                if nr >= self.world.players.len() {
                    self.world.players.resize(nr+1, Player::with_color(color));
                }
                self.world.players[nr].shape.color = color;
            },
            _ => bail!("Wrong message type."),

        };
        Ok(())
    }

    fn receive_world(&mut self, x: usize, y: usize, w: usize, h: usize, pixels: Vec<u8>) {
        let mut i = 0;
        for y in y..y+h {
            for x in x..x+w {
                self.world.tilenet.set(&pixels[i], (x, y));
                i += 1;
            }
        }
        self.graphics.tilenet_renderer.upload_texture(&self.world.tilenet, x as u32, y as u32, w as u32, h as u32);
    }
    

    fn handle_input(&mut self) -> Result<()> {
        if self.input.key_toggled_down(KeyCode::G) {
            self.socket.send_to(Message::ToggleGravity, self.server)?;
        }

        // Mouse
        if self.input.mouse() {
            // Update camera offset //
            let mut offset = self.input.mouse_moved() / self.zoom;
            offset.x = -offset.x;
            self.center += offset;

            // Fire weapon //
            let mouse_world_pos = screen_to_world(self.input.mouse_pos(),
                self.get_cam_pos(), 
                self.zoom,
                self.window_size.0,
                self.window_size.1);
            let dir = mouse_world_pos - self.world.players[self.player_nr].shape.pos;
            let msg = Message::BulletFire {direction: dir};
            self.socket.send_reliably_to(msg, self.server)?;
            info!("FIRING WEAPON"; "dir.x" => dir.x, "dir.y" => dir.y);
            info!("y"; "mouse" => mouse_world_pos.y, "player" => self.world.players[self.player_nr].shape.pos.y);
        }

        // Zooming
        const ZOOM_FACTOR: f32 = 1.2;
        let y = self.input.mouse_wheel();
        if y > 0.0 {
            self.zoom *= f32::powf(ZOOM_FACTOR, y as f32);
        } else if y < 0.0 {
            self.zoom /= f32::powf(ZOOM_FACTOR, -y as f32);
        }
        Ok(())
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


fn to_socket_addr(addr: &str) -> Result<SocketAddr> {
    // Assume IPv4. Try to parse.
    let parts: Vec<&str> = addr.split(":").collect();
    if parts.len() != 2 {
        bail!("IP address must be on the form X.X.X.X:port");
    }

    let addr: Vec<u8> = parts[0].split(".").map(|x| x.parse::<u8>().unwrap()).collect();
    if addr.len() != 4 {
        bail!("IP address must be on the form X.X.X.X:port");
    }

    let port = parts[1].parse::<u16>().unwrap();

    Ok(SocketAddr::V4(
        SocketAddrV4::new(
            Ipv4Addr::new(addr[0], addr[1], addr[2], addr[3]),
            port
        )
    ))
}
