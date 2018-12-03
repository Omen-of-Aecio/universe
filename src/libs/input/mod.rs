use glium::glutin;
use glium::glutin::{ElementState, MouseButton, VirtualKeyCode as KeyCode};
// use glium::glutin::KeyCode;
use glium::glutin::Event::KeyboardInput;
use libs::geometry::vec::Vec2;

const NUM_KEYS: usize = 150;
struct Keys([bool; NUM_KEYS]);

#[derive(Default)]
pub struct Input {
    key_down: Keys,
    key_toggled: Keys,

    left_mouse_button: bool,
    mouse: (i32, i32),
    mouse_in_previous_frame: (i32, i32),
    mouse_wheel: f32,
}

impl Default for Keys {
    fn default() -> Keys {
        Keys([false; 150])
    }
}

impl Input {

    pub fn prepare_for_next_frame(&mut self) {
        for i in 0..NUM_KEYS {
            self.key_toggled.0[i] = false;
        }
        self.mouse_wheel = 0.0;
        self.mouse_in_previous_frame.0 = self.mouse.0;
        self.mouse_in_previous_frame.1 = self.mouse.1;
    }

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
            self.left_mouse_button = match state {
                ElementState::Pressed => true,
                ElementState::Released => false,
            };
        }
    }

    // ---

    pub fn is_key_down(&self, keycode: KeyCode) -> bool {
        self.key_down.0[keycode as usize]
    }

    pub fn is_key_toggled(&self, keycode: KeyCode) -> bool {
        self.key_toggled.0[keycode as usize]
    }

    pub fn is_key_toggled_down(&self, keycode: KeyCode) -> bool {
        self.is_key_down(keycode) && self.is_key_toggled(keycode)
    }

    pub fn get_mouse_pos(&self) -> Vec2 {
        Vec2::new(self.mouse.0 as f32, self.mouse.1 as f32)
    }

    pub fn is_left_mouse_button_down(&self) -> bool {
        self.left_mouse_button
    }

    pub fn get_mouse_moved(&self) -> Vec2 {
        Vec2::new(
            (self.mouse.0 - self.mouse_in_previous_frame.0) as f32,
            (self.mouse.1 - self.mouse_in_previous_frame.1) as f32,
        )
    }

    pub fn get_mouse_wheel(&self) -> f32 {
        self.mouse_wheel
    }

    // ---

    pub fn register_key_down(&mut self, keycode: KeyCode) {
        let keycode = keycode as usize;
        if !self.key_down.0[keycode] {
            self.key_toggled.0[keycode] = true;
        }
        self.key_down.0[keycode] = true;
    }

    pub fn register_key_up(&mut self, keycode: KeyCode) {
        let keycode = keycode as usize;
        if self.key_down.0[keycode] {
            self.key_toggled.0[keycode] = true;
        }
        self.key_down.0[keycode] = false;
    }
}
