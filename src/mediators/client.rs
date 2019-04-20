use crate::glocals::*;
use crate::mediators::{
    does_line_collide_with_grid::*, vxdraw,
};
use benchmarker::Benchmarker;
use cgmath::*;
use geometry::{boxit::Boxit, cam::Camera, grid2d::Grid, vec::Vec2};
use input::Input;
use logger::{debug, info, InDebug, Logger};
use std::time::Instant;
use winit::{VirtualKeyCode as Key, *};

fn initialize_grid(s: &mut Grid<u8>) {
    s.resize(1000, 1000);
}

pub fn collect_input(client: &mut Client) {
    if let Some(ref mut windowing) = client.windowing {
        for event in super::vxdraw::collect_input(windowing) {
            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::KeyboardInput { input, .. } => {
                        client.input.register_key(&input);
                    }
                    WindowEvent::MouseWheel { delta, .. } => match delta {
                        winit::MouseScrollDelta::LineDelta(_, v) => {
                            client.input.register_mouse_wheel(v);
                        }
                        _ => {}
                    },
                    WindowEvent::MouseInput { state, button, .. } => {
                        client.input.register_mouse_input(state, button);
                    }
                    _ => {}
                },
                _ => {}
            }
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
            if s.game.cam.zoom < 5.0 {
                s.game.cam.zoom *= 1.1;
            }
        }
        x if x < 0.0 => {
            if s.game.cam.zoom > 0.01 {
                s.game.cam.zoom /= 1.1;
            }
        }
        _ => {}
    }
}

fn move_player_according_to_input(s: &Input) -> Vec2 {
    let (dx, dy);
    if s.is_key_down(Key::Up) {
        dy = -1;
    } else if s.is_key_down(Key::Down) {
        dy = 1;
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

fn check_player_collides_here(grid: &Grid<u8>, position: Vec2) -> bool {
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
    grid.get(tl.x as usize, tl.y as usize)
        .map_or(false, |x| *x > 0)
        || grid
            .get(tr.x as usize, tr.y as usize)
            .map_or(false, |x| *x > 0)
        || grid
            .get(br.x as usize, br.y as usize)
            .map_or(false, |x| *x > 0)
        || grid
            .get(bl.x as usize, bl.y as usize)
            .map_or(false, |x| *x > 0)
}

fn check_for_collision_and_move_player_according_to_movement_vector(
    grid: &Grid<u8>,
    player: &mut PlayerData,
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
    players: &mut [PlayerData],
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
        movement_current = Vec2 {
            x: movement.x,
            y: movement.y + 1.1f32,
        };
        let new_position = player.position + movement_current;
        if !check_player_collides_here(grid, new_position) {
            player.position += movement_current;
        }
    }
}

fn set_gravity(s: &mut Client) {
    if s.input.is_key_toggled_down(Key::G) {
        s.game.game_config.gravity_on = !s.game.game_config.gravity_on;
    }
}

fn create_black_square_around_player(s: &mut Grid<u8>) {
    for (i, j) in Boxit::with_center((100, 100), (500, 300)) {
        *s.get_mut(i, j).unwrap() = 0;
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

fn stop_benchmark(benchmarker: &mut Benchmarker, logger: &mut Logger<Log>, msg: &'static str) {
    if let Some(duration) = benchmarker.stop() {
        debug![logger, "benchmark", "{}", msg; "µs" => duration.as_micros() / 100];
    }
}

pub fn entry_point_client_vulkan(s: &mut Main) {
    if let Some(ref mut client) = s.client {
        client.logger.info("cli", "Creating grid");
        client.game.game_config.gravity = Vec2 { x: 0.0, y: -0.3 };
        client.game.cam.zoom = 0.01;

        client.logger.set_log_level(196);
        client
            .game
            .players2
            .push(PlayerData { position: Vec2 { x: 0.0, y: 0.0 } });

        let tex = vxdraw::strtex::push_texture(
            &mut client.windowing.as_mut().unwrap(),
            1000,
            1000,
            &mut client.logger,
        );
        client.game.grid.resize(1000, 1000);
        vxdraw::strtex::generate_map2(
            &mut client.windowing.as_mut().unwrap(),
            &tex,
            [1.0, 2.0, 4.0],
        );
        let grid = &mut client.game.grid;
        vxdraw::strtex::read(&mut client.windowing.as_mut().unwrap(), &tex, |x, pitch| {
            for j in 0..1000 {
                for i in 0..1000 {
                    grid.set(i, j, x[i + j * pitch].0);
                }
            }
        });
        vxdraw::strtex::push_sprite(
            &mut client.windowing.as_mut().unwrap(),
            &tex,
            vxdraw::strtex::Sprite {
                width: 1000.0,
                height: 1000.0,
                translation: (500.0, 500.0),
                ..vxdraw::strtex::Sprite::default()
            },
        );
        let handle = vxdraw::quads::push(
            &mut client.windowing.as_mut().unwrap(),
            vxdraw::quads::Quad {
                colors: [(255, 0, 0, 255); 4],
                width: 10.0,
                height: 10.0,
                origin: (-5.0, -5.0),
                ..vxdraw::quads::Quad::default()
            },
        );
        loop {
            s.time = Instant::now();
            let xform = if let Some(ref mut rx) = s.config_change_recv {
                match rx.try_recv() {
                    Ok(msg) => Some(msg),
                    Err(_) => None,
                }
            } else {
                None
            };
            if let Some(xform) = xform {
                xform(&mut s.config);
            }
            client_tick_vulkan(client, &handle);
            if let Some(ref mut network) = s.network {
                s.timers.network_timer.update(s.time, network);
            }
            if client.should_exit {
                break;
            }
        }
    }
}

fn upload_player_position(s: &mut Client, handle: &vxdraw::quads::QuadHandle) {
    let pos = s.game.players2[0].position;
    if let Some(ref mut windowing) = s.windowing {
        vxdraw::quads::set_position(windowing, handle, (pos.x, pos.y));
    }
}

fn client_tick_vulkan(s: &mut Client, handle: &vxdraw::quads::QuadHandle) {
    // ---
    s.logic_benchmarker.start();
    // ---

    collect_input(s);
    toggle_camera_mode(s);
    let movement = move_player_according_to_input(&s.input);
    check_for_collision_and_move_players_according_to_movement_vector(
        &s.game.grid,
        &mut s.game.players2,
        movement,
    );
    move_camera_according_to_input(s);

    upload_player_position(s, handle);

    // ---
    stop_benchmark(
        &mut s.logic_benchmarker,
        &mut s.logger,
        "Logic time spent (100-frame average)",
    );
    // ---

    // ---
    s.drawing_benchmarker.start();
    // ---

    if let Some(ref mut windowing) = s.windowing {
        let persp = super::vxdraw::utils::gen_perspective(windowing);
        let scale = Matrix4::from_scale(s.game.cam.zoom);
        let center = s.game.cam.center;
        // let lookat = Matrix4::look_at(Point3::new(center.x, center.y, -1.0), Point3::new(center.x, center.y, 0.0), Vector3::new(0.0, 0.0, -1.0));
        let trans = Matrix4::from_translation(Vector3::new(-center.x / 10.0, center.y / 10.0, 0.0));
        // info![client.logger, "main", "Okay wth"; "trans" => InDebug(&trans); clone trans];
        super::vxdraw::draw_frame(windowing, &mut s.logger, &(persp * scale * trans));
    }

    // ---
    stop_benchmark(
        &mut s.drawing_benchmarker,
        &mut s.logger,
        "Drawing time spent (100-frame average)",
    );
    // ---

    std::thread::sleep(std::time::Duration::new(0, 8_000_000));
}
