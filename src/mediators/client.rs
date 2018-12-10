use crate::glocals::{CameraMode, Client, GridU8RenderData, PolygonRenderData};
use crate::libs::geometry::{cam::Camera, grid2d::Grid, vec::Vec2};
use crate::libs::input::Input;
use crate::mediators::{
    does_line_collide_with_grid::*, logger::log, random_map_generator, render_grid, render_polygon,
};
use glium::{
    self,
    glutin::{self, MouseScrollDelta, VirtualKeyCode as Key},
    Surface,
};
use time::PreciseTime;

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
    } / if s.is_key_down(Key::LShift) { 3.0 } else { 1.0 }
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

fn render_players(players: &mut [PolygonRenderData], frame: &mut glium::Frame, cam: &Camera) {
    for player in players {
        render_polygon::render(player, frame, cam);
    }
}

fn check_player_collides_here(
    grid: &Grid<u8>,
    position: Vec2,
) -> bool {
    let tl = Vec2 {
        x: position.x + 0.01,
        y: position.y + 0.01,
    };
    let tr = Vec2 {
        x: position.x + 9.99,
        y: position.y + 0.01,
    };
    let bl = Vec2 {
        x: position.x + 0.01,
        y: position.y + 9.99,
    };
    let br = Vec2 {
        x: position.x + 9.99,
        y: position.y + 9.99,
    };
    grid.get(tl.x as usize, tl.y as usize).map_or(false, |x| *x > 0)
    || grid.get(tr.x as usize, tr.y as usize).map_or(false, |x| *x > 0)
    || grid.get(br.x as usize, br.y as usize).map_or(false, |x| *x > 0)
    || grid.get(bl.x as usize, bl.y as usize).map_or(false, |x| *x > 0)
}

fn check_for_collision_and_move_player_according_to_movement_vector(
    grid: &Grid<u8>,
    player: &mut PolygonRenderData,
    movement: Vec2,
) -> bool {
    let tl = Vec2 {
        x: player.position.x + 0.01,
        y: player.position.y + 0.01,
    };
    let tr = Vec2 {
        x: player.position.x + 9.99,
        y: player.position.y + 0.01,
    };
    let bl = Vec2 {
        x: player.position.x + 0.01,
        y: player.position.y + 9.99,
    };
    let br = Vec2 {
        x: player.position.x + 9.99,
        y: player.position.y + 9.99,
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
        return false;
    }
    true
}

fn check_for_collision_and_move_players_according_to_movement_vector(
    grid: &Grid<u8>,
    players: &mut [PolygonRenderData],
    movement: Vec2,
) {
    for player in players {
        let mut movement_current = movement;
        let collided = check_for_collision_and_move_player_according_to_movement_vector(
            grid,
            player,
            movement_current,
        );
        if !collided {
            break;
        }
        movement_current = Vec2 { x: movement.x, y : movement.y + 1.1f32 };
        let new_position = player.position + movement_current;
        if !check_player_collides_here(grid, new_position) {
            player.position += movement_current;
        }
    }
}

fn apply_gravity_to_players_2(grid: &Grid<u8>, players: &mut [PolygonRenderData], velocity: Vec2) {
    for player in players {
        check_for_collision_and_move_player_according_to_movement_vector(grid, player, velocity);
    }
}

fn apply_gravity_to_players(s: &mut Client) {
    apply_gravity_to_players_2(
        &s.game.grid,
        &mut s.game.players,
        s.game.game_config.gravity,
    );
}

fn set_gravity(s: &mut Client) {
    if s.input.is_key_toggled_down(Key::G) {
        s.game.game_config.gravity_on = !s.game.game_config.gravity_on;
    }
}

fn set_smooth(s: &mut Client) {
    if s.input.is_key_toggled_down(Key::R) {
        if let Some(ref mut gridrenderdata) = s.game.grid_render {
            render_grid::toggle_smooth(gridrenderdata);
            log(
                &mut s.main.threads,
                128,
                "CLNT",
                "Toggling grid smoothing",
                &[("smooth", &format!["{}", gridrenderdata.smooth])],
            );
        }
    }
}

fn create_black_square_around_player(s: &mut Grid<u8>) {
    for j in 200..400 {
        for i in 400..600 {
            *s.get_mut(i, j).unwrap() = 0;
        }
    }
}

fn set_camera(s: &mut Client) {
    match s.game.cam_mode {
        CameraMode::Interactive => {}
        CameraMode::FollowPlayer => {
            let center = s.game.players[0].position;
            s.game.cam.center = center;
        }
    }
}

fn toggle_camera_mode(s: &mut Client) {
    if s.input.is_key_toggled_down(Key::F) {
        s.game.cam_mode = match s.game.cam_mode {
            CameraMode::FollowPlayer => CameraMode::Interactive,
            CameraMode::Interactive => CameraMode::FollowPlayer,
        };
    }
}

pub fn entry_point_client(s: &mut Client) {
    log(&mut s.main.threads, 128, "MAIN", "Creating grid", &[]);
    initialize_grid(&mut s.game.grid);
    s.game.game_config.gravity = Vec2 { x: 0.0, y: -0.3 };
    random_map_generator::proc1(&mut s.game.grid, &s.display);
    create_black_square_around_player(&mut s.game.grid);
    // let size = s.game.grid.get_size();
    // for i in 0 .. size.0 {
    //     *s.game.grid.get_mut(i, 800).unwrap() = 255;
    //     *s.game.grid.get_mut(i, 0).unwrap() = 255;
    //     *s.game.grid.get_mut(40, i).unwrap() = 255;
    //     *s.game.grid.get_mut(600, i).unwrap() = 255;
    // }
    // *s.game.grid.get_mut(100, 1).unwrap() = 255;
    s.game.grid_render = Some(render_grid::create_grid_u8_render_data(
        &s.display,
        &s.game.grid,
    ));
    s.game
        .players
        .push(render_polygon::create_render_polygon(&s.display));
    let mut frame_counter = 0u8;
    let mut time_spent = time::Duration::zero();
    loop {
        let begin = PreciseTime::now();

        // ---

        collect_input(s);
        if s.should_exit {
            break;
        }
        toggle_camera_mode(s);
        let movement = move_player_according_to_input(&s.input);
        check_for_collision_and_move_players_according_to_movement_vector(
            &s.game.grid,
            &mut s.game.players,
            movement,
        );
        move_camera_according_to_input(s);
        set_camera(s);
        set_gravity(s);
        set_smooth(s);
        if s.game.game_config.gravity_on {
            apply_gravity_to_players(s);
        }

        let mut frame = s.display.draw();
        frame.clear_color(0.0, 0.0, 1.0, 1.0);
        render_the_grid(&mut s.game.grid_render, &mut frame, &s.game.cam);
        render_players(&mut s.game.players, &mut frame, &s.game.cam);

        // ---

        if frame_counter >= 100 {
            log(
                &mut s.main.threads,
                128 + 64,
                "CLNT",
                "Time spent, 100-frame average",
                &[(
                    "µs",
                    &format!["{:?}", time_spent.num_microseconds().map(|x| x / 100)],
                )],
            );
            frame_counter = 0;
            time_spent = time::Duration::zero();
        } else {
            let end = PreciseTime::now();
            let elapsed = begin.to(end);
            time_spent = time_spent + elapsed;
            frame_counter += 1;
        }

        // ---

        match frame.finish() {
            Ok(()) => {}
            Err(glium::SwapBuffersError::ContextLost) => {
                log(
                    &mut s.main.threads,
                    64,
                    "CLNT",
                    "Context was lost while trying to swap buffers",
                    &[],
                );
            }
            Err(glium::SwapBuffersError::AlreadySwapped) => {
                log(
                    &mut s.main.threads,
                    64,
                    "CLNT",
                    "OpenGL context has already been swapped",
                    &[],
                );
            }
        }
    }
}
