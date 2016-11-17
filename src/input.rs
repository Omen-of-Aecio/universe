use glium::glutin;
use glium::glutin::ElementState;
// use glium::glutin::VirtualKeyCode;
use glium::glutin::Event::{KeyboardInput};

const NUM_KEYS: usize = 150;

pub struct Input {
    key_down: [bool; NUM_KEYS],
    key_toggled: [bool; NUM_KEYS]
}


impl Input {
    pub fn new() -> Input {
        Input {
            key_down: [false; NUM_KEYS],
            key_toggled: [false; NUM_KEYS],
        }
    }

    // Mainly resets key_toggled
    pub fn update(&mut self) {
        for i in 0..NUM_KEYS {
            self.key_toggled[i] = false;
        }
    }

    pub fn register_key(&mut self, input: glutin::Event) {
        match input {
            KeyboardInput(ElementState::Pressed, _, Some(keycode))
                => self.register_key_down(keycode),
            KeyboardInput(ElementState::Released, _, Some(keycode))
                => self.register_key_up(keycode),
            _
                => () // Do nothing. Should probably log the error.
        }
    }

    pub fn key_down(&self, keycode: glutin::VirtualKeyCode) -> bool {
        return self.key_down[keycode as usize];
    }

    pub fn key_toggled(&self, keycode: glutin::VirtualKeyCode) -> bool {
        return self.key_toggled[keycode as usize];
    }


    fn register_key_down(&mut self, keycode: glutin::VirtualKeyCode) {
        println!("Key down: {}", keycode as i32);
        let keycode = keycode as usize;
        if !self.key_down[keycode] {
            // If this toggles the key...
            self.key_toggled[keycode] = true;
        }
        self.key_down[keycode] = true;
    }
    fn register_key_up(&mut self, keycode: glutin::VirtualKeyCode) {
        let keycode = keycode as usize;
        if self.key_down[keycode] {
            // If this toggles the key...
            self.key_toggled[keycode] = true;
        }
        self.key_down[keycode] = false;
    }
}
