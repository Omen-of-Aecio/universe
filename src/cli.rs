use geometry::vec::Vec2;
use glium;
use glium::{DisplayBuild, glutin};
use glium::glutin::{ElementState, MouseButton, MouseScrollDelta};
use input::Input;
use world::World;
use graphics::Graphics;
use graphics::screen_to_world;
use err::{Result, Error};
use rand;
use rand::Rng;

use net::{Message, Socket};
use std::net::{SocketAddr, SocketAddrV4, Ipv4Addr};

const CLIENT_PORT: u16 = 10123;


/* Should go, together with some logic, to some camera module (?) */
enum CameraMode {
    Interactive,
    FollowPlayer,
}

pub struct Client {
    display: glium::Display,
    input: Input,
    graphics: Graphics,
    world: World,

    // Camera & input (for now)
    cam_mode: CameraMode,
    //   following is used only if INTERACTIVE camera mode
    zoom: f32,
    center: Vec2,
    mouse_down: bool,
    mouse_pos: Vec2,
    mouse_pos_past: Vec2,

    // Networking
    socket: Socket,
    server: SocketAddr,

    a: i32,
}



impl Client {
    pub fn run(&mut self) -> Result<()> {
        let mut window_size = self.display.get_window().unwrap().get_inner_size().unwrap();
        let mut oldpos = Vec2::null_vec();
        loop {
            println!("Number of world pieces: {}", self.a);
            // Input
            self.input.update();
            // Handle input events
            for ev in self.display.clone().poll_events() {
                match ev {
                    glutin::Event::Closed => return Ok(()),
                    glutin::Event::MouseMoved(x, y) => self.mouse_moved(x, y),
                    glutin::Event::MouseWheel(MouseScrollDelta::LineDelta(_, y), _) => {
                        self.mouse_wheel_line(y)
                    }
                    glutin::Event::MouseInput(ElementState::Pressed, button) => {
                        self.mouse_press(button)
                    }
                    glutin::Event::MouseInput(ElementState::Released, button) => {
                        self.mouse_release(button)
                    }
                    glutin::Event::KeyboardInput(_, _, _) => self.input.register_key(ev),
                    glutin::Event::Resized(w, h) => window_size = (w, h),
                    _ => (),
                }
            }
            // Networking
            for msg in &mut self.socket.messages().unwrap() {
                match msg {
                    Ok((src, msg)) => {
                        self.handle_message(src, msg)?;
                    },
                    Err(e) => return Err(e),
                }
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
            let cam_pos = match self.cam_mode {
                CameraMode::Interactive => self.center,
                CameraMode::FollowPlayer => self.world.get_cam_pos(),
            };
            prof!["Render",
                  self.graphics.render(cam_pos,
                                       self.zoom,
                                       window_size.0,
                                       window_size.1,
                                       &self.world)];
            
            // vsync doesn't seem to work on Windows
            // thread::sleep(Duration::from_millis(15));
        }
    }
    /// Currently just ignores unexpected messages
    fn handle_message(&mut self, src: SocketAddr, msg: Message) -> Result<()> {
        if src != self.server {
            return Err(Error::Other("Packet not from server".to_string()));
        }
        match msg {
            Message::WorldMeta {width:_, height:_} => {
            },
            Message::WorldRect {x, y, width, height, pixels} => {
                if width * height != pixels.len() {
                    return Err(Error::Other(format!("Not enough pixels ({}) to cover rect ({}, {}; {}, {})", pixels.len(), x, y, width, height)));
                }
                self.receive_world(x, y, width, height, pixels);
            },
            _ => { println!("CLIENT: WRONG PACKAGE"); return Err(Error::Other("Wrong message type.".to_string()));},
        };
        Ok(())
    }

    fn receive_world(&mut self, x: usize, y: usize, w: usize, h: usize, pixels: Vec<u8>) {
        self.a += 1;
        assert!(pixels.len() == w*h);
        let mut i = 0;
        for y in y..y+h {
            for x in x..x+w {
                self.world.tilenet.set(&pixels[i], (x, y));
                i += 1;
            }
        }
        self.graphics.tilenet_renderer.upload_texture(&self.world.tilenet, x as u32, y as u32, w as u32, h as u32);
    }

    pub fn new(server_addr: &str) -> Result<Client> {

        let mut socket = Client::create_socket();
        let server = to_socket_addr(server_addr);

        // Init connection
        socket.send_to(Message::Join, server);
        // Get world metadata
        let (src, msg) = socket.recv()?;
        // TODO reordering will be problematic here, expecting only a certain message
        let (w, h) = match msg {
            Message::WorldMeta {width, height} => (width, height),
            _ => return Err(Error::Other("hello".to_string())),
        };

        println!("Client creation. World size = ({}, {})", w, h);

        // let pos = Vec2::new(WORLD_SIZE as f32 - 50.0, WORLD_SIZE as f32/3.0);
        let pos = Vec2::new(w as f32 / 2.0, h as f32/2.0);
        let mut world = World::new(w, h, pos);

        let display = glutin::WindowBuilder::new().build_glium().unwrap();
        let graphics = Graphics::new(display.clone(), &world);
        Ok(Client {
            display: display,
            input: Input::new(),
            graphics: graphics,
            world: world,
            cam_mode: CameraMode::FollowPlayer,
            zoom: 1.0,
            center: Vec2::new(0.0, 0.0),
            mouse_down: false,
            mouse_pos: Vec2::new(0.0, 0.0),
            mouse_pos_past: Vec2::new(0.0, 0.0),

            socket: socket,
            server: server,
            a: 0,
        })
    }

    fn mouse_moved(&mut self, x: i32, y: i32) {
        self.mouse_pos_past = self.mouse_pos;
        self.mouse_pos = Vec2::new(x as f32, y as f32);
        // Move the texture //
        if self.mouse_down {
            // let window_size = self.display.get_window().unwrap().get_inner_size().unwrap();
            let mut offset = (self.mouse_pos - self.mouse_pos_past) / self.zoom;
            offset.x = -offset.x;
            offset.y = offset.y;
            self.center += offset;
        }
    }

    fn mouse_wheel_line(&mut self, y: f32) {
        // For each 'tick', it should *= factor
        const ZOOM_FACTOR: f32 = 1.2;
        if y > 0.0 {
            self.zoom *= f32::powf(ZOOM_FACTOR, y as f32);
        } else if y < 0.0 {
            self.zoom /= f32::powf(ZOOM_FACTOR, -y as f32);
        }
    }

    fn mouse_press(&mut self, button: MouseButton) {
        if let MouseButton::Left = button {
            self.mouse_down = true;
        }
    }

    fn mouse_release(&mut self, button: MouseButton) {
        if let MouseButton::Left = button {
            self.mouse_down = false;
        }
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


fn to_socket_addr(addr: &str) -> SocketAddr{
    // Assume IPv4. Try to parse.
    let parts: Vec<&str> = addr.split(":").collect();
    assert!(parts.len() == 2);

    let addr: Vec<u8> = parts[0].split(".").map(|x| x.parse::<u8>().unwrap()).collect();
    assert!(addr.len() == 4);

    let port = parts[1].parse::<u16>().unwrap();

    SocketAddr::V4(
        SocketAddrV4::new(
            Ipv4Addr::new(addr[0], addr[1], addr[2], addr[3]),
            port
        )
    )
}
