use glium::{
    self,
    glutin::{self, MouseScrollDelta},
    Display, DisplayBuild, Surface,
};
use glocals::Client;
use libs::geometry::grid2d::Grid;
use mediators::{logger::log, random_map_generator, render_grid};

fn initialize_grid(s: &mut Grid<u8>) {
    s.resize(1000, 1000);
}

pub fn collect_input(client: &mut Client) {
    client.input.prepare_for_next_frame();
    for ev in client.display.poll_events() {
        match ev {
            glutin::Event::Closed => {
                return;
            }
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
            _ => {}
        }
    }
}

fn move_camera_according_to_input(s: &mut Client) {
    use glium::glutin::VirtualKeyCode as Key;
    if s.input.is_key_down(Key::D) {
        s.game.cam.center.x += 1.0;
    }
    if s.input.is_key_down(Key::A) {
        s.game.cam.center.x -= 1.0;
    }
    if s.input.is_key_down(Key::W) {
        s.game.cam.center.y += 1.0;
    }
    if s.input.is_key_down(Key::S) {
        s.game.cam.center.y -= 1.0;
    }
}

pub fn entry_point_client(s: &mut Client) {
    log(&mut s.main.threads, 128, "MAIN", "Creating grid", &[]);
    initialize_grid(&mut s.game.grid);
    random_map_generator::proc1(&mut s.game.grid);
    s.game.grid_render = Some(render_grid::create_grid_u8_render_data(&s.display, &s.game.grid));
    loop {
        collect_input(s);
        move_camera_according_to_input(s);
        let mut frame = s.display.draw();
        frame.clear_color(0.0, 0.0, 0.0, 1.0);
        let (x, y) = (s.game.cam.center.x, s.game.cam.center.y);
        s.game.grid_render.as_mut().map(|s| render_grid::render(s, &mut frame, (x, y), 1.0, 1000, 1000));
        frame.finish();
    }
}
