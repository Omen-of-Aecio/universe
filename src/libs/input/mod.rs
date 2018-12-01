use glium::glutin;
use glium::glutin::{ElementState, MouseButton, VirtualKeyCode as KeyCode};
// use glium::glutin::KeyCode;
use glium::glutin::Event::KeyboardInput;
use glocals::component::PlayerInput;
use libs::geometry::vec::Vec2;

// TODO
// Input isn't really made for ease of client-server

const NUM_KEYS: usize = 150;
struct Keys([bool; NUM_KEYS]);

#[derive(Default)]
pub struct Input {
    key_down: Keys,
    key_toggled: Keys,

    /// Only left mouse button at the moment
    mouse: (i32, i32, bool),
    /// How much the mouse has moved since the last frame
    past_mouse: (i32, i32),
    /// How much wheel since last frame
    mouse_wheel: f32,
}

impl Default for Keys {
    fn default() -> Keys {
        Keys([false; 150])
    }
}

impl Input {
    pub fn new() -> Input {
        Input::default()
    }

    // Mainly resets key_toggled.0
    pub fn update(&mut self) {
        for i in 0..NUM_KEYS {
            self.key_toggled.0[i] = false;
        }
        self.mouse_wheel = 0.0;
        self.past_mouse.0 = self.mouse.0;
        self.past_mouse.1 = self.mouse.1;
    }

    /* Interface to register input */
    pub fn register_key(&mut self, input: &glutin::Event) {
        match *input {
            KeyboardInput(ElementState::Pressed, _, Some(keycode)) => {
                self.register_key_down(keycode)
            }
            KeyboardInput(ElementState::Released, _, Some(keycode)) => {
                self.register_key_up(keycode)
            }
            _ => (), // Do nothing. Should probably log the error.
        }
    }
    pub fn position_mouse(&mut self, x: i32, y: i32) {
        self.mouse.0 = x;
        self.mouse.1 = y;
    }
    pub fn register_mouse_wheel(&mut self, y: f32) {
        self.mouse_wheel = y;
    }
    pub fn register_mouse_input(&mut self, state: ElementState, button: MouseButton) {
        if let MouseButton::Left = button {
            self.mouse.2 = match state {
                ElementState::Pressed => true,
                ElementState::Released => false,
            };
        }
    }

    /* Interface to GET state */

    pub fn key_down(&self, keycode: KeyCode) -> bool {
        self.key_down.0[keycode as usize]
    }

    pub fn key_toggled(&self, keycode: KeyCode) -> bool {
        self.key_toggled.0[keycode as usize]
    }

    /// True if key was just pressed down this frame.
    pub fn key_toggled_down(&self, keycode: KeyCode) -> bool {
        self.key_down(keycode) && self.key_toggled(keycode)
    }

    pub fn mouse_pos(&self) -> Vec2 {
        Vec2::new(self.mouse.0 as f32, self.mouse.1 as f32)
    }
    pub fn mouse(&self) -> bool {
        self.mouse.2
    }
    pub fn mouse_moved(&self) -> Vec2 {
        Vec2::new(
            (self.mouse.0 - self.past_mouse.0) as f32,
            (self.mouse.1 - self.past_mouse.1) as f32,
        )
    }
    pub fn mouse_wheel(&self) -> f32 {
        self.mouse_wheel
    }

    pub fn register_key_down(&mut self, keycode: KeyCode) {
        // debug!("Key down"; "code" => keycode as i32);
        let keycode = keycode as usize;
        if !self.key_down.0[keycode] {
            // If this toggles the key...
            self.key_toggled.0[keycode] = true;
        }
        self.key_down.0[keycode] = true;
    }
    pub fn register_key_up(&mut self, keycode: KeyCode) {
        let keycode = keycode as usize;
        if self.key_down.0[keycode] {
            // If this toggles the key...
            self.key_toggled.0[keycode] = true;
        }
        self.key_down.0[keycode] = false;
    }

    pub fn create_player_input(&self) -> PlayerInput {
        PlayerInput {
            up: self.key_down(KeyCode::Up)
                || self.key_down(KeyCode::W)
                || self.key_down(KeyCode::F),
            down: self.key_down(KeyCode::Down)
                || self.key_down(KeyCode::S)
                || self.key_down(KeyCode::S),
            left: self.key_down(KeyCode::Left)
                || self.key_down(KeyCode::A)
                || self.key_down(KeyCode::R),
            right: self.key_down(KeyCode::Right)
                || self.key_down(KeyCode::D)
                || self.key_down(KeyCode::T),
            g: self.key_down(KeyCode::G),
        }
    }
}
