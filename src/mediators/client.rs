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

pub fn entry_point_client(s: &mut Client) {
    log(&mut s.main.threads, 128, "MAIN", "Creating grid", &[]);
    initialize_grid(&mut s.game.grid);
    random_map_generator::proc1(&mut s.game.grid);
    let mut renderer = render_grid::create_grid_u8_render_data(&s.display, &s.game.grid);
    let size = s.game.grid.get_size();
    for j in 0..size.1 {
        for i in 0..size.0 {
            print![
                "{}",
                if *s.game.grid.get(i, j).unwrap() > 0 {
                    0
                } else {
                    1
                }
            ];
        }
        println![""];
    }
    loop {
        collect_input(s);
        let mut frame = s.display.draw();
        frame.clear_color(0.0, 0.0, 0.0, 1.0);
        render_grid::render(&mut renderer, &mut frame, (500.0, 300.0), 1.0, 1000, 1000);
        frame.finish();
    }
}
