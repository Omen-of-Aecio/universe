use err::*;
use glium::glutin;
use glium::glutin::{MouseScrollDelta, VirtualKeyCode as KeyCode};
use glium::DisplayBuild;
use global::Tile;
use glocals::{game::Game, *};
use graphics::Graphics;
use input::Input;
use net::msg::Message;
use net::{to_socket_addr, Socket};
use rand;
use rand::Rng;
use specs::DispatcherBuilder;
use srv::system::MaintainSys;
use std::net::SocketAddr;
use tilenet::TileNet;

impl Client {
    pub fn new(server_addr: &str) -> Result<Client, Error> {
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
                let mut game = Game::new(width, height, you, white_base, black_base, &display);
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
}

pub fn run(client: &mut Client) -> Result<(), Error> {
    client.game.cam.update_win_size(&client.display);

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
        let packets = client.game.update(&mut dispatcher, &client.input);
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
            s.game.apply_snapshot(snapshot);
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
