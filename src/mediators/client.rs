use crate::glocals::{vxdraw::*, *};
use crate::mediators::vxdraw::*;
use crate::mediators::{does_line_collide_with_grid::*, vxdraw};
use benchmarker::Benchmarker;
use cgmath::*;
use geometry::{boxit::Boxit, cam::Camera, grid2d::Grid, vec::Vec2};
use input::Input;
use logger::{debug, info, InDebug, Logger};
use rand::Rng;
use std::time::Instant;
use winit::{VirtualKeyCode as Key, *};

static FIREBALLS: &[u8] = include_bytes!["../../assets/images/Fireball_68x9.png"];

fn initialize_grid(s: &mut Grid<u8>) {
    s.resize(1000, 1000);
}

pub fn collect_input(client: &mut Logic, windowing: &mut Windowing) {
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
                WindowEvent::CursorMoved { position, .. } => {
                    let pos: (i32, i32) = position.to_physical(1.6666).into();
                    client.input.position_mouse(pos.0, pos.1);
                }
                _ => {}
            },
            _ => {}
        }
    }
}

fn move_camera_according_to_input(s: &mut Logic) {
    if s.input.is_key_down(Key::D) {
        s.cam.center.x += 5.0;
    }
    if s.input.is_key_down(Key::A) {
        s.cam.center.x -= 5.0;
    }
    if s.input.is_key_down(Key::W) {
        s.cam.center.y += 5.0;
    }
    if s.input.is_key_down(Key::S) {
        s.cam.center.y -= 5.0;
    }
    match s.input.get_mouse_wheel() {
        x if x > 0.0 => {
            if s.cam.zoom < 5.0 {
                s.cam.zoom *= 1.1;
            }
        }
        x if x < 0.0 => {
            if s.cam.zoom > 0.002 {
                s.cam.zoom /= 1.1;
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

fn set_gravity(s: &mut Logic) {
    if s.input.is_key_toggled_down(Key::G) {
        s.game_config.gravity_on = !s.game_config.gravity_on;
    }
}

fn create_black_square_around_player(s: &mut Grid<u8>) {
    for (i, j) in Boxit::with_center((100, 100), (500, 300)) {
        s.set(i, j, 0);
    }
}

fn toggle_camera_mode(s: &mut Logic) {
    if s.input.is_key_toggled_down(Key::F) {
        s.cam_mode = match s.cam_mode {
            CameraMode::FollowPlayer => CameraMode::Interactive,
            CameraMode::Interactive => CameraMode::FollowPlayer,
        };
    }
}

pub fn maybe_initialize_graphics(s: &mut Main) {
    let mut windowing = init_window_with_vulkan(&mut s.logger, ShowWindow::Enable);

    {
        static BACKGROUND: &[u8] = include_bytes!["../../assets/images/terrabackground.png"];
        let background = dyntex::push_texture(
            &mut windowing,
            BACKGROUND,
            dyntex::TextureOptions {
                depth_test: true,
                fixed_perspective: Some(Matrix4::identity()),
                ..dyntex::TextureOptions::default()
            },
        );
        dyntex::push_sprite(
            &mut windowing,
            &background,
            dyntex::Sprite {
                depth: 1.0,
                ..dyntex::Sprite::default()
            },
        );
    }

    let tex = vxdraw::strtex::push_texture(&mut windowing, 1000, 1000, &mut s.logger);
    s.logic.grid.resize(1000, 1000);
    vxdraw::strtex::generate_map2(&mut windowing, &tex, [1.0, 2.0, 4.0]);
    let grid = &mut s.logic.grid;
    vxdraw::strtex::read(&mut windowing, &tex, |x, pitch| {
        for j in 0..1000 {
            for i in 0..1000 {
                grid.set(i, j, x[i + j * pitch].0);
            }
        }
    });
    vxdraw::strtex::push_sprite(
        &mut windowing,
        &tex,
        vxdraw::strtex::Sprite {
            width: 1000.0,
            height: 1000.0,
            translation: (500.0, 500.0),
            ..vxdraw::strtex::Sprite::default()
        },
    );
    let handle = vxdraw::quads::push(
        &mut windowing,
        vxdraw::quads::Quad {
            colors: [(255, 0, 0, 255); 4],
            width: 10.0,
            height: 10.0,
            origin: (-5.0, -5.0),
            ..vxdraw::quads::Quad::default()
        },
    );

    let fireballs = vxdraw::dyntex::push_texture(
        &mut windowing,
        FIREBALLS,
        vxdraw::dyntex::TextureOptions::default(),
    );
    s.graphics = Some(Graphics {
        player_quads: vec![handle],
        bullets_texture: fireballs,
        grid: tex,
        windowing,
    });
}

fn update_graphics(s: &mut Main) {
    if let Some(ref mut graphics) = s.graphics {
        let changeset = &s.logic.changed_tiles;
        vxdraw::strtex::streaming_texture_set_pixels(
            &mut graphics.windowing,
            &graphics.grid,
            changeset
                .iter()
                .map(|pos| (pos.0 as u32, pos.1 as u32, (0, 0, 0, 255))),
        );

        vxdraw::dyntex::set_uvs2(
            &mut graphics.windowing,
            s.logic.bullets.iter().map(|b| {
                (
                    b.handle.as_ref().unwrap(),
                    b.current_uv_begin,
                    b.current_uv_end,
                )
            }),
        );

        for b in s.logic.bullets.iter() {
            vxdraw::dyntex::set_position(
                &mut graphics.windowing,
                b.handle.as_ref().unwrap(),
                b.position.into(),
            );
        }

        upload_player_position(
            &mut s.logic,
            &mut graphics.windowing,
            &graphics.player_quads[0],
        );
    }
    s.logic.changed_tiles.clear();
}

fn draw_graphics(s: &mut Main) {
    if let Some(ref mut graphics) = s.graphics {
        let persp = super::vxdraw::utils::gen_perspective(&mut graphics.windowing);
        let scale = Matrix4::from_scale(s.logic.cam.zoom);
        let center = s.logic.cam.center;
        // let lookat = Matrix4::look_at(Point3::new(center.x, center.y, -1.0), Point3::new(center.x, center.y, 0.0), Vector3::new(0.0, 0.0, -1.0));
        let trans = Matrix4::from_translation(Vector3::new(-center.x, center.y, 0.0));
        // info![client.logger, "main", "Okay wth"; "trans" => InDebug(&trans); clone trans];
        super::vxdraw::draw_frame(
            &mut graphics.windowing,
            &mut s.logger,
            &(persp * scale * trans),
        );
        collect_input(&mut s.logic, &mut graphics.windowing);
    }
}

pub fn entry_point_client(s: &mut Main) {
    s.logger.set_log_level(196);

    s.logic.game_config.gravity = Vec2 { x: 0.0, y: -0.3 };
    s.logic.cam.zoom = 0.01;

    s.logic.players.push(PlayerData {
        position: Vec2 { x: 0.0, y: 0.0 },
    });

    s.logger.info("cli", "Initializing graphics");
    maybe_initialize_graphics(s);

    initialize_grid(&mut s.logic.grid);
    create_black_square_around_player(&mut s.logic.grid);

    let mut draw_bench = benchmarker::Benchmarker::new(100);
    let mut update_bench = benchmarker::Benchmarker::new(100);

    loop {
        s.time = Instant::now();
        tick_logic(&mut s.logic, &mut s.logger);
        update_bullets_position(&mut s.logic, s.graphics.as_mut().map(|x| &mut x.windowing));

        {
            let wheel = s.logic.input.get_mouse_wheel();
            match wheel {
                x if x == 0.0 => {}
                x if x > 0.0 => {
                    s.logic.current_weapon = Weapon::Ak47;
                }
                x if x < 0.0 => {
                    s.logic.current_weapon = Weapon::Hellfire;
                }
                _ => {}
            }
        }
        s.timers.network_timer.update(s.time, &mut s.network);
        if s.logic.should_exit {
            break;
        }
        fire_bullets(&mut s.logic, &mut s.graphics, &mut s.random);

        let ((), duration) = update_bench.run(|| {
            update_graphics(s);
        });
        if let Some(duration) = duration {
            info![s.logger, "cli", "Time taken per update"; "duration" => InDebug(&duration)];
        }

        let ((), duration) = draw_bench.run(|| {
            draw_graphics(s);
        });
        if let Some(duration) = duration {
            info![s.logger, "cli", "Time taken per graphics"; "duration" => InDebug(&duration)];
        }
    }
}

fn upload_player_position(
    s: &mut Logic,
    windowing: &mut Windowing,
    handle: &vxdraw::quads::QuadHandle,
) {
    let pos = s.players[0].position;
    vxdraw::quads::set_position(windowing, handle, (pos.x, pos.y));
}

fn fire_bullets(s: &mut Logic, graphics: &mut Option<Graphics>, random: &mut rand_pcg::Pcg64Mcg) {
    if s.input.is_left_mouse_button_down() {
        let weapon = &s.current_weapon;

        let spread = if weapon == &Weapon::Hellfire {
            0.1
        } else {
            0.3
        };

        let direction = if let Some(ref mut graphics) = graphics {
            (Vec2::from(s.input.get_mouse_pos())
                - Vec2::from(graphics.windowing.get_window_size_in_pixels_float()) / 2.0)
                .rotate(random.gen_range(-spread, spread))
        } else {
            Vec2 { x: 1.0, y: 0.0 }
        };

        let handle = if let Some(ref mut graphics) = graphics {
            Some(vxdraw::dyntex::push_sprite(
                &mut graphics.windowing,
                &graphics.bullets_texture,
                vxdraw::dyntex::Sprite {
                    width: 6.8,
                    height: 0.9,
                    scale: 3.0,
                    rotation: -direction.angle() + std::f32::consts::PI,
                    ..vxdraw::dyntex::Sprite::default()
                },
            ))
        } else {
            None
        };

        let position = s.players.get(0).map_or(Vec2 { x: 0.0, y: 0.0 }, |x| {
            x.position + Vec2 { x: 5.0, y: 5.0 }
        });
        s.bullets.push(Bullet {
            direction: direction.normalize(),
            position,

            animation_sequence: 0,
            animation_block_begin: (0.0, 0.0),
            animation_block_end: (1.0, 1.0),
            height: 6,
            width: 10,
            current_uv_begin: (0.0, 0.0),
            current_uv_end: (0.0, 0.0),
            handle,
        });
    }
}

fn update_bullets_uv(s: &mut Logic) {
    for b in s.bullets.iter_mut() {
        let width_elem = b.animation_sequence % b.width;
        let height_elem = b.animation_sequence / b.width;
        let uv_begin = (
            width_elem as f32 / b.width as f32,
            height_elem as f32 / b.height as f32,
        );
        let uv_end = (
            (width_elem + 1) as f32 / b.width as f32,
            (height_elem + 1) as f32 / b.height as f32,
        );
        b.animation_sequence += 1;
        if b.animation_sequence > b.width * b.height {
            b.animation_sequence = 0;
        }
        b.current_uv_begin = (Vec2::from(uv_begin) + Vec2::from(b.animation_block_begin)).into();
        b.current_uv_end = (Vec2::from(uv_end) * Vec2::from(b.animation_block_end)).into();
    }
}

fn update_bullets_position(s: &mut Logic, mut windowing: Option<&mut Windowing>) {
    let mut bullets_to_remove = vec![];
    for (idx, b) in s.bullets.iter_mut().enumerate() {
        if let Some(pos) =
            does_line_collide_with_grid(&s.grid, b.position, b.position + b.direction, |x| {
                *x == 255
            })
        {
            bullets_to_remove.push(idx);
            for i in -3..=3 {
                for j in -3..=3 {
                    let pos = (pos.0 as i32 + i, pos.1 as i32 + j);
                    let pos = (pos.0 as usize, pos.1 as usize);
                    s.grid.set(pos.0, pos.1, 0);
                    s.changed_tiles.push((pos.0, pos.1));
                }
            }
        }
        b.position += b.direction;
    }

    use std::cmp::Ordering;
    bullets_to_remove.sort_by(|x, y| {
        if *x < *y {
            Ordering::Greater
        } else if *x == *y {
            Ordering::Equal
        } else {
            Ordering::Less
        }
    });

    for idx in bullets_to_remove.drain(..) {
        let bullet = s.bullets.swap_remove(idx);
        if let Some(ref mut windowing) = windowing {
            if let Some(handle) = bullet.handle {
                dyntex::remove_sprite(windowing, handle);
            }
        }
    }
}

fn tick_logic(s: &mut Logic, logger: &mut Logger<Log>) {
    toggle_camera_mode(s);
    let movement = move_player_according_to_input(&s.input);
    check_for_collision_and_move_players_according_to_movement_vector(
        &s.grid,
        &mut s.players,
        movement,
    );
    move_camera_according_to_input(s);
    update_bullets_uv(s);
    std::thread::sleep(std::time::Duration::new(0, 8_000_000));
}
