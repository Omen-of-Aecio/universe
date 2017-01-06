use glium::glutin;
use glium::glutin::{ElementState, VirtualKeyCode as KeyCode};
// use glium::glutin::KeyCode;
use glium::glutin::Event::KeyboardInput;

// TODO
// Input isn't really made for ease of client-server


const NUM_KEYS: usize = 150;

pub struct Input {
    key_down: [bool; NUM_KEYS],
    key_toggled: [bool; NUM_KEYS],
}

impl Default for Input {
    fn default() -> Input {
        Input {
            key_down: [false; NUM_KEYS],
            key_toggled: [false; NUM_KEYS],
        }
    }
}

impl Input {
    pub fn new() -> Input {
        Input::default()
    }

    // Mainly resets key_toggled
    pub fn update(&mut self) {
        for i in 0..NUM_KEYS {
            self.key_toggled[i] = false;
        }
    }

    pub fn register_key(&mut self, input: glutin::Event) {
        match input {
            KeyboardInput(ElementState::Pressed, _, Some(keycode)) => {
                self.register_key_down(keycode)
            }
            KeyboardInput(ElementState::Released, _, Some(keycode)) => {
                self.register_key_up(keycode)
            }
            _ => (), // Do nothing. Should probably log the error.
        }
    }

    pub fn key_down(&self, keycode: KeyCode) -> bool {
        self.key_down[keycode as usize]
    }

    pub fn key_toggled(&self, keycode: KeyCode) -> bool {
        self.key_toggled[keycode as usize]
    }

    /// True if key was just pressed down this frame.
    pub fn key_toggled_down(&self, keycode: KeyCode) -> bool {
        self.key_down(keycode) && self.key_toggled(keycode)
    }


    pub fn register_key_down(&mut self, keycode: KeyCode) {
        debug!("Key down"; "code" => keycode as i32);
        let keycode = keycode as usize;
        if !self.key_down[keycode] {
            // If this toggles the key...
            self.key_toggled[keycode] = true;
        }
        self.key_down[keycode] = true;
    }
    pub fn register_key_up(&mut self, keycode: KeyCode) {
        let keycode = keycode as usize;
        if self.key_down[keycode] {
            // If this toggles the key...
            self.key_toggled[keycode] = true;
        }
        self.key_down[keycode] = false;
    }

    pub fn create_player_input(&self) -> PlayerInput {
        PlayerInput {
            up: self.key_down(KeyCode::Up) || self.key_down(KeyCode::W) || self.key_down(KeyCode::F),
            down: self.key_down(KeyCode::Down) || self.key_down(KeyCode::S) || self.key_down(KeyCode::S),
            left: self.key_down(KeyCode::Left) || self.key_down(KeyCode::A) || self.key_down(KeyCode::R),
            right: self.key_down(KeyCode::Right) || self.key_down(KeyCode::D) || self.key_down(KeyCode::T),
            g: self.key_down(KeyCode::G),
        }
    }
}

#[derive(Debug, Default, Copy, Clone, RustcEncodable, RustcDecodable)]
pub struct PlayerInput {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub g: bool,
}
