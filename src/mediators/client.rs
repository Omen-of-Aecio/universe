use crate::glocals::*;
use crate::mediators::does_line_collide_with_grid::*;
use cgmath::*;
use fast_logger::{info, GenericLogger, Logger};
use geometry::{boxit::Boxit, grid2d::Grid, vec::Vec2};
use input::Input;
use rand::Rng;
use std::time::Instant;
use vxdraw::*;
use winit::{VirtualKeyCode as Key, *};

static FIREBALLS: &dyntex::ImgData =
    &dyntex::ImgData::PNGBytes(include_bytes!["../../assets/images/bullets.png"]);
static WEAPONS: &dyntex::ImgData =
    &dyntex::ImgData::PNGBytes(include_bytes!["../../assets/images/weapons.png"]);

fn initialize_grid(s: &mut Grid<u8>) {
    s.resize(1000, 1000);
}

pub fn collect_input(client: &mut Logic, windowing: &mut VxDraw) {
    for event in windowing.collect_input() {
        if let Event::WindowEvent { event, .. } = event {
            match event {
                WindowEvent::KeyboardInput { input, .. } => {
                    client.input.register_key(&input);
                }
                WindowEvent::MouseWheel {
                    delta, modifiers, ..
                } => {
                    if let winit::MouseScrollDelta::LineDelta(_, v) = delta {
                        client.input.register_mouse_wheel(v);
                        if modifiers.ctrl {
                            client.input.set_ctrl();
                        }
                    }
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    client.input.register_mouse_input(state, button);
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let pos: (i32, i32) = position.to_physical(1.6666).into();
                    client.input.position_mouse(pos.0, pos.1);
                }
                _ => {}
            }
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
        s.cam.center.y -= 5.0;
    }
    if s.input.is_key_down(Key::S) {
        s.cam.center.y += 5.0;
    }
    if s.input.get_ctrl() {
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

    if s.cam_mode == CameraMode::FollowPlayer {
        if let Some(player) = s.players.get_mut(0) {
            s.cam.center -= (s.cam.center - player.position) / 10.0;
        }
    }
}

fn accelerate_player_according_to_input(s: &Input, conf: &Config, on_ground: bool) -> Vec2 {
    let dy = if s.is_key_down(Key::Up) {
        if conf.world.gravity_on {
            if on_ground {
                -conf.player.jump_acc
            } else {
                0.0
            }
        } else {
            -conf.player.acc
        }
    } else if s.is_key_down(Key::Down) {
        conf.player.acc
    } else {
        0.0
    };
    let dx = if s.is_key_down(Key::Left) {
        -conf.player.acc
    } else if s.is_key_down(Key::Right) {
        conf.player.acc
    } else {
        0.0
    };
    Vec2 {
        x: dx as f32,
        y: dy as f32,
    } / if s.is_key_down(Key::LShift) { 3.0 } else { 1.0 }
}

/// Returns true if collision happened on y axis
fn check_for_collision_and_move_player_according_to_movement_vector(
    grid: &Grid<u8>,
    player: &mut PlayerData,
    movement: Vec2,
    _logger: &mut Logger<Log>,
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
    let mut collision_y = None;
    let ymove = Vec2 {
        x: 0.0,
        y: movement.y,
    };
    for i in 1..=10 {
        collision_y = collision_test(&[tl, tr, br, bl], Some(0.5), ymove / i as f32, grid, |x| {
            *x > 0
        });
        if collision_y.is_none() {
            player.position += ymove / i as f32;
            break;
        }
    }

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
    let mut collision_x = None;
    let xmove = Vec2 {
        x: movement.x,
        y: 0.0,
    };
    for i in 1..=10 {
        collision_x = collision_test(&[tl, tr, br, bl], Some(0.5), xmove / i as f32, grid, |x| {
            *x > 0
        });
        if collision_x.is_none() {
            player.position += xmove / i as f32;
            break;
        }
    }
    if collision_x.is_some() {
        player.velocity.x = 0.0;
    }
    if collision_y.is_some() {
        player.velocity.y = 0.0;
    }
    collision_y.is_some()
}

fn set_gravity(s: &mut Logic) {
    if s.input.is_key_toggled_down(Key::G) {
        s.config.world.gravity_on = !s.config.world.gravity_on;
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
    let mut windowing = VxDraw::new(s.logger.clone().to_compatibility(), ShowWindow::Enable);

    {
        static BACKGROUND: &dyntex::ImgData =
            &dyntex::ImgData::PNGBytes(include_bytes!["../../assets/images/terrabackground.png"]);
        let background = windowing.dyntex().add_layer(
            BACKGROUND,
            &dyntex::LayerOptions::new()
                .depth(true)
                .fixed_perspective(Matrix4::identity()),
        );
        windowing.dyntex().add(&background, dyntex::Sprite::new());
    }

    let mut strtex = windowing.strtex();

    let tex = strtex.add_layer(
        &strtex::LayerOptions::new()
            .width(1000)
            .height(1000)
            .depth(false),
    );
    s.logic.grid.resize(1000, 1000);

    strtex.fill_with_perlin_noise(&tex, [1.0, 2.0, 4.0]);
    let grid = &mut s.logic.grid;
    strtex.read(&tex, |x, pitch| {
        for j in 0..1000 {
            for i in 0..1000 {
                grid.set(i, j, x[i + j * pitch].0);
            }
        }
    });
    strtex.add(
        &tex,
        vxdraw::strtex::Sprite::new()
            .width(1000.0)
            .height(1000.0)
            .translation((500.0, 500.0)),
    );
    let layer = windowing
        .quads()
        .add_layer(&vxdraw::quads::LayerOptions::default());
    let handle = windowing.quads().add(
        &layer,
        vxdraw::quads::Quad::new()
            .colors([(255, 0, 0, 127); 4])
            .width(10.0)
            .height(10.0)
            .origin((-5.0, -5.0)),
    );

    let mut dyntex = windowing.dyntex();

    let fireballs = dyntex.add_layer(FIREBALLS, &vxdraw::dyntex::LayerOptions::new());

    let weapons_texture = dyntex.add_layer(WEAPONS, &vxdraw::dyntex::LayerOptions::new());
    let text_layer = windowing.text().add_layer(
        include_bytes!["../../crates/vxdraw/fonts/DejaVuSans.ttf"],
        vxdraw::text::LayerOptions::new(),
    );
    let basic_text = windowing.text().add(
        &text_layer,
        "( ͡° ͜ʖ ͡°)",
        vxdraw::text::TextOptions::new()
            .font_size(40.0)
            .scale(100.0)
            .translation((110.0, 3.2)),
    );

    s.graphics = Some(Graphics {
        basic_text,
        player_quads: vec![handle],
        bullets_texture: fireballs,
        grid: tex,
        weapons_texture,
        windowing,
    });
}

fn update_graphics(s: &mut Main) {
    if let Some(ref mut graphics) = s.graphics {
        let changeset = &s.logic.changed_tiles;
        graphics.windowing.strtex().set_pixels(
            &graphics.grid,
            changeset
                .iter()
                .map(|pos| (pos.0 as u32, pos.1 as u32, Color::Rgba(0, 0, 0, 255))),
        );

        graphics
            .windowing
            .dyntex()
            .set_uvs(s.logic.bullets.iter().map(|b| {
                (
                    b.handle.as_ref().unwrap(),
                    b.current_uv_begin,
                    b.current_uv_end,
                )
            }));

        for b in s.logic.bullets.iter() {
            graphics
                .windowing
                .dyntex()
                .set_translation(b.handle.as_ref().unwrap(), b.position.into());
        }

        {
            let angle = -(Vec2::from(s.logic.input.get_mouse_pos())
                - Vec2::from(graphics.windowing.get_window_size_in_pixels_float()) / 2.0)
                .angle();
            if let Some(Some(sprite)) = s.logic.players.get_mut(0).map(|x| &mut x.weapon_sprite) {
                if angle > std::f32::consts::PI / 2.0 || angle < -std::f32::consts::PI / 2.0 {
                    graphics
                        .windowing
                        .dyntex()
                        .set_uv(sprite, (0.0, 1.0), (1.0, 0.0));
                } else {
                    graphics
                        .windowing
                        .dyntex()
                        .set_uv(sprite, (0.0, 0.0), (1.0, 1.0));
                }
                graphics.windowing.dyntex().set_rotation(sprite, Rad(angle));
            }
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
        let persp = vxdraw::utils::gen_perspective(&graphics.windowing);
        let scale = Matrix4::from_scale(s.logic.cam.zoom);
        let center = s.logic.cam.center;
        // let lookat = Matrix4::look_at(Point3::new(center.x, center.y, -1.0), Point3::new(center.x, center.y, 0.0), Vector3::new(0.0, 0.0, -1.0));
        let trans = Matrix4::from_translation(Vector3::new(-center.x, -center.y, 0.0));
        // info![client.logger, "main", "Okay wth"; "trans" => InDebug(&trans); clone trans];
        graphics.windowing.draw_frame(&(persp * scale * trans));
    }
}

pub fn entry_point_client(s: &mut Main) {
    s.logger.set_log_level(196);

    s.logic.cam.zoom = 0.01;

    s.logic.players.push(PlayerData {
        position: Vec2::null_vec(),
        velocity: Vec2::null_vec(),
        weapon_sprite: None,
    });

    s.logger.info("cli", "Initializing graphics");
    maybe_initialize_graphics(s);

    initialize_grid(&mut s.logic.grid);
    create_black_square_around_player(&mut s.logic.grid);

    let port = s.network.get_port();
    info![s.logger, "main", "Listening on port"; "port" => port];

    loop {
        s.time = Instant::now();
        tick_logic(s);
        if s.logic.should_exit {
            break;
        }
    }
}

fn upload_player_position(s: &mut Logic, windowing: &mut VxDraw, handle: &vxdraw::quads::Handle) {
    if let Some(ref mut player) = s.players.get(0) {
        if let Some(ref gun_handle) = player.weapon_sprite {
            windowing.dyntex().set_translation(
                gun_handle,
                (player.position + Vec2 { x: 5.0, y: 5.0 }).into(),
            );
        }
        windowing
            .quads()
            .set_solid_color(handle, Color::Rgba(0, 255, 0, 255));
        windowing
            .quads()
            .set_translation(handle, player.position.into());
    }
}

fn fire_bullets(s: &mut Logic, graphics: &mut Option<Graphics>, random: &mut rand_pcg::Pcg64Mcg) {
    if s.input.is_left_mouse_button_down() {
        if s.current_weapon_cooldown == 0 {
            s.current_weapon_cooldown = match s.current_weapon {
                Weapon::Hellfire => 5,
                Weapon::Ak47 => 2,
            }
        } else {
            s.current_weapon_cooldown -= 1;
            return;
        }

        let weapon = &s.current_weapon;

        let spread = if weapon == &Weapon::Hellfire {
            0.3
        } else {
            0.1
        };

        let (
            width,
            height,
            animation_block_begin,
            animation_block_end,
            sprite_width,
            sprite_height,
            destruction,
            bullet_count,
            speed,
        ) = match weapon {
            Weapon::Hellfire => (10, 6, (0.0, 0.0), (1.0, 53.0 / 60.0), 6.8, 0.9, 3, 1, 1.0),
            Weapon::Ak47 => (
                1,
                1,
                (0.0, 54.0 / 60.0),
                (4.0 / 679.0, 58.0 / 60.0),
                0.5,
                0.5,
                1,
                1,
                2.0,
            ),
        };

        for _ in 0..bullet_count {
            let direction = if let Some(ref mut graphics) = graphics {
                (Vec2::from(s.input.get_mouse_pos())
                    - Vec2::from(graphics.windowing.get_window_size_in_pixels_float()) / 2.0)
                    .rotate(random.gen_range(-spread, spread))
            } else {
                Vec2 { x: 1.0, y: 0.0 }
            };

            let handle = if let Some(ref mut graphics) = graphics {
                Some(
                    graphics.windowing.dyntex().add(
                        &graphics.bullets_texture,
                        vxdraw::dyntex::Sprite::new()
                            .width(sprite_width)
                            .height(sprite_height)
                            .scale(3.0)
                            .origin((-sprite_width / 2.0, sprite_height / 2.0))
                            .rotation(Rad(-direction.angle() + std::f32::consts::PI)),
                    ),
                )
            } else {
                None
            };

            let position = s.players.get(0).map_or(Vec2 { x: 0.0, y: 0.0 }, |x| {
                x.position + Vec2 { x: 5.0, y: 5.0 }
            });
            s.bullets.push(Bullet {
                direction: direction.normalize() * speed,
                position,
                destruction,

                animation_sequence: 0,
                animation_block_begin,
                animation_block_end,
                height,
                width,
                current_uv_begin: (0.0, 0.0),
                current_uv_end: (0.0, 0.0),
                handle,
            });
        }
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
        if b.animation_sequence >= b.width * b.height {
            b.animation_sequence = 0;
        }
        let current_uv_begin = (Vec2::from(uv_begin) * Vec2::from(b.animation_block_end)
            + Vec2::from(b.animation_block_begin))
        .into();
        let current_uv_end = (Vec2::from(uv_end) * Vec2::from(b.animation_block_end)).into();
        b.current_uv_begin = current_uv_begin;
        b.current_uv_end = current_uv_end;
    }
}

fn update_bullets_position(s: &mut Logic, mut windowing: Option<&mut VxDraw>) {
    let mut bullets_to_remove = vec![];
    for (idx, b) in s.bullets.iter_mut().enumerate() {
        let collision = collision_test(&[b.position], None, b.direction, &s.grid, |x| *x == 255);
        if let Some((xi, yi)) = collision {
            bullets_to_remove.push(idx);
            let area = b.destruction;
            for i in -area..=area {
                for j in -area..=area {
                    let pos = (xi as i32 + i, yi as i32 + j);
                    let pos = (pos.0 as usize, pos.1 as usize);
                    s.grid.set(pos.0, pos.1, 0);
                    s.changed_tiles.push((pos.0, pos.1));
                }
            }
        } else {
            b.position += b.direction;
        }
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
                windowing.dyntex().remove(handle);
            }
        }
    }
}
fn apply_physics_to_players(s: &mut Logic, logger: &mut Logger<Log>) {
    for player in &mut s.players {
        if s.config.world.gravity_on {
            player.velocity += Vec2::new(0.0, s.config.world.gravity);
        }

        let on_ground = check_for_collision_and_move_player_according_to_movement_vector(
            &s.grid,
            player,
            player.velocity,
            logger,
        );
        player.velocity += accelerate_player_according_to_input(&s.input, &s.config, on_ground);
        player.velocity = player.velocity.clamp(Vec2 {
            x: s.config.player.max_vel,
            y: s.config.player.max_vel,
        });
        if on_ground {
            player.velocity.x *= s.config.world.ground_fri;
        } else {
            player.velocity.x *= s.config.world.air_fri_x;
        }
        player.velocity.y *= s.config.world.air_fri_y;
    }
}

pub fn tick_logic(s: &mut Main) {
    toggle_camera_mode(&mut s.logic);

    apply_physics_to_players(&mut s.logic, &mut s.logger);
    move_camera_according_to_input(&mut s.logic);
    update_bullets_uv(&mut s.logic);
    std::thread::sleep(std::time::Duration::new(0, 8_000_000));

    set_gravity(&mut s.logic);
    update_bullets_position(&mut s.logic, s.graphics.as_mut().map(|x| &mut x.windowing));

    if let Some(Ok(msg)) = s.threads.game_shell_channel.as_mut().map(|x| x.try_recv()) {
        (msg)(s);
    }

    let wheel = s.logic.input.get_mouse_wheel();
    match wheel {
        x if x == 0.0 => {}
        x if x > 0.0 => {
            s.logic.current_weapon = Weapon::Ak47;
            if let Some(this_player) = s.logic.players.get_mut(0) {
                if let Some(ref mut gfx) = s.graphics {
                    let new = gfx.windowing.dyntex().add(
                        &gfx.weapons_texture,
                        dyntex::Sprite::new().width(10.0).height(5.0),
                    );
                    let old = std::mem::replace(&mut this_player.weapon_sprite, Some(new));
                    if let Some(old_id) = old {
                        gfx.windowing.dyntex().remove(old_id);
                    }
                }
            }
        }
        x if x < 0.0 => {
            s.logic.current_weapon = Weapon::Hellfire;
        }
        _ => {}
    }

    s.timers.network_timer.update(s.time, &mut s.network);
    let mut buffer = [0u8; 100];
    if let Ok((_sender, msg)) = s.network.recv(&mut buffer) {
        info![s.logger, "main", "Got a message from the network"; "content" => msg];
    }

    fire_bullets(&mut s.logic, &mut s.graphics, &mut s.random);

    update_graphics(s);

    s.logic.input.prepare_for_next_frame();
    if let Some(ref mut graphics) = s.graphics {
        collect_input(&mut s.logic, &mut graphics.windowing);
    }

    draw_graphics(s);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mediators::testtools::*;

    // ---

    #[test]
    fn basic_setup_and_teardown() {
        Main::default();
    }

    #[test]
    fn basic_setup_gsh() {
        let mut main = Main::default();
        spawn_gameshell(&mut main);
        assert![main.threads.game_shell_channel.is_some()];
        assert_eq!["6", gsh(&mut main, "+ 1 2 3")];
    }

    #[test]
    fn gsh_change_gravity() {
        let mut main = Main::default();
        spawn_gameshell(&mut main);
        assert_eq![
            "Set gravity value",
            gsh(&mut main, "config gravity set y 1.23")
        ];
        tick_logic(&mut main);
        assert_eq![1.23, main.logic.config.world.gravity];
    }

    #[test]
    fn gsh_change_gravity_synchronous() {
        let mut main = Main::default();
        spawn_gameshell(&mut main);
        assert_eq![
            "Set gravity value",
            gsh_synchronous(&mut main, "config gravity set y 1.23", tick_logic)
        ];
        assert_eq![1.23, main.logic.config.world.gravity];
    }

    #[test]
    fn gsh_get_fps() {
        let mut main = Main::default();
        spawn_gameshell(&mut main);
        assert_eq![
            "0",
            gsh_synchronous(&mut main, "config fps get", tick_logic)
        ];

        gsh(&mut main, "config fps set 1.23");
        tick_logic(&mut main);

        assert_eq![
            "1.23",
            gsh_synchronous(&mut main, "config fps get", tick_logic)
        ];
    }
}
