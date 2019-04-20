use crate::glocals::*;
use crate::mediators::{
    does_line_collide_with_grid::*, random_map_generator, render_grid, render_polygon, vxdraw,
};
use benchmarker::Benchmarker;
use cgmath::*;
use geometry::{boxit::Boxit, cam::Camera, grid2d::Grid, vec::Vec2};
use glium::{
    self,
    glutin::{self, MouseScrollDelta},
    Surface,
};
use input::Input;
use logger::{debug, info, InDebug, Logger};
use std::time::Instant;
use winit::{VirtualKeyCode as Key, *};

fn initialize_grid(s: &mut Grid<u8>) {
    s.resize(1000, 1000);
}

pub fn collect_input_vk(client: &mut Client) {
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

pub fn collect_input(client: &mut Client) {
    client.input.prepare_for_next_frame();
    for ev in client.display.poll_events() {
        match ev {
            glutin::Event::Closed => {
                client.should_exit = true;
            }
            // glutin::Event::MouseMoved(x, y) => client.input.position_mouse(x, y),
            glutin::Event::MouseWheel(MouseScrollDelta::LineDelta(_, y), _) => {
                // client.input.register_mouse_wheel(y)
            }
            glutin::Event::MouseInput(state, button) => {
                // client.input.register_mouse_input(state, button)
            }
            // glutin::Event::KeyboardInput(_, _, _) => client.input.register_key(&ev),
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

fn render_the_grid(grid: &mut Option<GridU8RenderData>, frame: &mut glium::Frame, cam: &Camera) {
    if let Some(ref mut grid) = grid {
        render_grid::render(
            grid,
            frame,
            (cam.center.x, cam.center.y),
            cam.zoom,
            cam.width,
            cam.height,
        );
    }
}

fn render_players(players: &mut [PolygonRenderData], frame: &mut glium::Frame, cam: &Camera) {
    for player in players {
        render_polygon::render(player, frame, cam);
    }
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

fn check_for_collision_and_move_player_according_to_movement_vector_vk(
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

fn check_for_collision_and_move_players_according_to_movement_vector_vk(
    grid: &Grid<u8>,
    players: &mut [PlayerData],
    movement: Vec2,
) {
    for player in players {
        let mut movement_current = movement;
        let collided = check_for_collision_and_move_player_according_to_movement_vector_vk(
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

fn apply_gravity_to_players_2(grid: &Grid<u8>, players: &mut [PolygonRenderData], velocity: Vec2) {
    for player in players {
        check_for_collision_and_move_player_according_to_movement_vector(grid, player, velocity);
    }
}

fn apply_gravity_to_players(s: &mut Client) {
    if s.game.game_config.gravity_on {
        apply_gravity_to_players_2(
            &s.game.grid,
            &mut s.game.players,
            s.game.game_config.gravity,
        );
    }
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
            let smooth = gridrenderdata.smooth;
            logger::info!(s.logger, "cli", "Toggling grid smoothing"; "smooth" => smooth);
        }
    }
}

fn create_black_square_around_player(s: &mut Grid<u8>) {
    for (i, j) in Boxit::with_center((100, 100), (500, 300)) {
        *s.get_mut(i, j).unwrap() = 0;
    }
}

fn set_camera(s: &mut Client) {
    match s.game.cam_mode {
        CameraMode::Interactive => {}
        CameraMode::FollowPlayer => {
            let center = s.game.players[0].position;
            let cam_center = s.game.cam.center;
            s.game.cam.center += (center - cam_center) / 10.0;
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

fn maybe_fire_bullets(s: &mut Client) {
    if s.input.is_left_mouse_button_down() {
        let mouse = s.input.get_mouse_pos();
        let world = s.game.cam.screen_to_world(mouse.into());
        s.logger.trace("cli", Log::Coordinates(world, mouse.into()));
        let target = s.game.cam.screen_to_world(s.input.get_mouse_pos().into());
        let origin = s.game.players[0].position + Vec2 { x: 5.0, y: 5.0 };
        let length = (target - origin).length_squared();
        if length > 0.1 {
            let bullet = Bullet {
                render: render_polygon::create_render_polygon(&s.display),
                direction: (target - origin).normalize(),
                position: origin,
            };
            s.game.bullets.push(bullet);
        }
    }
}

fn render_bullets(bullets: &[Bullet], frame: &mut glium::Frame, cam: &Camera) {
    for bullet in bullets {
        render_polygon::render(&bullet.render, frame, &cam);
    }
}

fn update_bullets(bullets: &mut Vec<Bullet>) {
    for bullet in bullets {
        bullet.position += bullet.direction * 3.0;
        bullet.render.position = bullet.position;
    }
}

// fn play_song(s: &mut Client) {
//     use std::{fs::File, io::BufReader};
//     let file = File::open("assets/sound/Kitsune^2 - Rainbow Tylenol.mp3").unwrap();
//     let source = rodio::Decoder::new(BufReader::new(file)).unwrap();
//     s.audio.append(source);
// }

fn remove_bullets_outside_camera(_log: &mut Logger<Log>, bullets: &mut Vec<Bullet>, cam: &Camera) {
    let bocs = cam.get_view_bocs();
    let mut x = vec![];
    for (i, bullet) in bullets.iter().enumerate() {
        if !bocs.is_point_inside(bullet.position) {
            x.push(i);
        }
    }
    for i in x.iter().rev() {
        bullets.remove(*i);
    }
}

fn stop_benchmark(benchmarker: &mut Benchmarker, logger: &mut Logger<Log>, msg: &'static str) {
    if let Some(duration) = benchmarker.stop() {
        debug![logger, "benchmark", "{}", msg; "µs" => duration.as_micros() / 100];
    }
}

pub fn entry_point_client(s: &mut Main) {
    // play_song(s);
    if let Some(ref mut client) = s.client {
        client.logger.info("cli", "Creating grid");
        initialize_grid(&mut client.game.grid);
        client.game.game_config.gravity = Vec2 { x: 0.0, y: -0.3 };
        random_map_generator::proc1(&mut client.game.grid, &client.display);
        create_black_square_around_player(&mut client.game.grid);
        // let size = s.game.grid.get_size();
        // for i in 0 .. size.0 {
        //     *s.game.grid.get_mut(i, 800).unwrap() = 255;
        //     *s.game.grid.get_mut(i, 0).unwrap() = 255;
        //     *s.game.grid.get_mut(40, i).unwrap() = 255;
        //     *s.game.grid.get_mut(600, i).unwrap() = 255;
        // }
        // *s.game.grid.get_mut(100, 1).unwrap() = 255;
        client.game.grid_render = Some(render_grid::create_grid_u8_render_data(
            &client.display,
            &client.game.grid,
        ));
        client
            .game
            .players
            .push(render_polygon::create_render_polygon(&client.display));

        client.logger.set_log_level(196);
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
            client_tick(client);
            if let Some(ref mut network) = s.network {
                s.timers.network_timer.update(s.time, network);
            }
            if client.should_exit {
                break;
            }
        }
    }
}

fn client_tick(s: &mut Client) {
    // ---
    s.logic_benchmarker.start();
    // ---

    collect_input(s);
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
    apply_gravity_to_players(s);
    maybe_fire_bullets(s);
    update_bullets(&mut s.game.bullets);
    remove_bullets_outside_camera(&mut s.logger, &mut s.game.bullets, &s.game.cam);

    // ---
    stop_benchmark(
        &mut s.logic_benchmarker,
        &mut s.logger,
        "Logic time spent (100-frame average)",
    );
    // ---

    let mut frame = s.display.draw();
    frame.clear_color(0.0, 0.0, 1.0, 1.0);

    // ---
    s.drawing_benchmarker.start();
    // ---

    render_the_grid(&mut s.game.grid_render, &mut frame, &s.game.cam);
    render_players(&mut s.game.players, &mut frame, &s.game.cam);
    render_bullets(&s.game.bullets, &mut frame, &s.game.cam);

    // ---
    stop_benchmark(
        &mut s.drawing_benchmarker,
        &mut s.logger,
        "Drawing time spent (100-frame average)",
    );
    // ---

    match frame.finish() {
        Ok(()) => {}
        Err(glium::SwapBuffersError::ContextLost) => {
            s.logger
                .error("cli", "Context was lost while trying to swap buffers");
        }
        Err(glium::SwapBuffersError::AlreadySwapped) => {
            s.logger
                .error("cli", "OpenGL context has already been swapped");
        }
    }
}

pub fn entry_point_client_vulkan(s: &mut Main) {
    if let Some(ref mut client) = s.client {
        client.logger.info("cli", "Creating grid");
        client.game.game_config.gravity = Vec2 { x: 0.0, y: -0.3 };

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

    collect_input_vk(s);
    toggle_camera_mode(s);
    let movement = move_player_according_to_input(&s.input);
    check_for_collision_and_move_players_according_to_movement_vector_vk(
        &s.game.grid,
        &mut s.game.players2,
        movement,
    );
    move_camera_according_to_input(s);
    set_camera(s);
    set_gravity(s);
    set_smooth(s);
    apply_gravity_to_players(s);
    maybe_fire_bullets(s);
    update_bullets(&mut s.game.bullets);
    remove_bullets_outside_camera(&mut s.logger, &mut s.game.bullets, &s.game.cam);

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
