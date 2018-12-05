use crate::glocals::{Client, GridU8RenderData, PolygonRenderData};
use crate::libs::geometry::{cam::Camera, grid2d::Grid, vec::Vec2};
use crate::libs::input::Input;
use crate::mediators::{
    does_line_collide_with_grid::*, logger::log, random_map_generator, render_grid, render_polygon,
};
use glium::{
    self,
    glutin::{self, MouseScrollDelta, VirtualKeyCode as Key},
    Display, DisplayBuild, Surface,
};

fn initialize_grid(s: &mut Grid<u8>) {
    s.resize(1000, 1000);
}

pub fn collect_input(client: &mut Client) {
    client.input.prepare_for_next_frame();
    for ev in client.display.poll_events() {
        match ev {
            glutin::Event::Closed => {
                client.should_exit = true;
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
    if s.input.is_key_down(Key::D) {
        s.game.cam.center.x += 5.0;
    }
    if s.input.is_key_down(Key::A) {
        s.game.cam.center.x -= 5.0;
    }
    if s.input.is_key_down(Key::W) {
        s.game.cam.center.y += 5.0;
    }
    if s.input.is_key_down(Key::S) {
        s.game.cam.center.y -= 5.0;
    }
    match s.input.get_mouse_wheel() {
        x if x > 0.0 => {
            s.game.cam.zoom *= 1.1 / 1.0;
        }
        x if x < 0.0 => {
            s.game.cam.zoom *= 1.0 / 1.1;
        }
        _ => {}
    }
}

fn move_player_according_to_input(s: &Input) -> Vec2 {
    let (dx, dy);
    if s.is_key_down(Key::Up) {
        dy = 1;
    } else if s.is_key_down(Key::Down) {
        dy = -1;
    } else {
        dy = 0;
    }
    if s.is_key_down(Key::Left) {
        dx = -1;
    } else if s.is_key_down(Key::Right) {
        dx = 1;
    } else {
        dx = 0;
    }
    Vec2 {
        x: dx as f32,
        y: dy as f32,
    }
}

fn render_the_grid(grid: &mut Option<GridU8RenderData>, frame: &mut glium::Frame, cam: &Camera) {
    grid.as_mut().map(|s| {
        render_grid::render(
            s,
            frame,
            (cam.center.x, cam.center.y),
            cam.zoom,
            cam.width,
            cam.height,
        )
    });
}

fn render_players(
    players: &mut [PolygonRenderData],
    display: &Display,
    frame: &mut glium::Frame,
    cam: &Camera,
) {
    for player in players {
        render_polygon::render(player, display, frame, cam);
    }
}

fn check_for_collision_and_move_players_according_to_movement_vector(
    grid: &Grid<u8>,
    players: &mut [PolygonRenderData],
    movement: Vec2,
) {
    for player in players {
        let tl = Vec2 {
            x: player.position.x,
            y: player.position.y,
        };
        let tr = Vec2 {
            x: player.position.x + 9.0,
            y: player.position.y,
        };
        let bl = Vec2 {
            x: player.position.x,
            y: player.position.y + 9.0,
        };
        let br = Vec2 {
            x: player.position.x + 9.0,
            y: player.position.y + 9.0,
        };
        let collision_point = do_lines_collide_with_grid(
            grid,
            &[
                (tl, tl + movement),
                (tr, tr + movement),
                (bl, bl + movement),
                (br, br + movement),
            ],
            |x| *x > 0,
        );
        if collision_point.is_none() {
            player.position.x += movement.x as f32;
            player.position.y += movement.y as f32;
        }
    }
}

pub fn entry_point_client(s: &mut Client) {
    log(&mut s.main.threads, 128, "MAIN", "Creating grid", &[]);
    initialize_grid(&mut s.game.grid);
    random_map_generator::proc1(&mut s.game.grid, &s.display);
    s.game.grid_render = Some(render_grid::create_grid_u8_render_data(
        &s.display,
        &s.game.grid,
    ));
    s.game
        .players
        .push(render_polygon::create_render_polygon(&s.display));
    loop {
        collect_input(s);
        if s.should_exit {
            break;
        }
        move_camera_according_to_input(s);
        let movement = move_player_according_to_input(&s.input);
        check_for_collision_and_move_players_according_to_movement_vector(
            &s.game.grid,
            &mut s.game.players,
            movement,
        );
        let mut frame = s.display.draw();
        frame.clear_color(0.0, 0.0, 1.0, 1.0);
        render_the_grid(&mut s.game.grid_render, &mut frame, &s.game.cam);
        render_players(&mut s.game.players, &s.display, &mut frame, &s.game.cam);
        frame.finish();
    }
}
