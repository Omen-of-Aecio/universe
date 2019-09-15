use winit::*;

const NUM_KEYS: usize = 161;
struct Keys([KeyboardInput; NUM_KEYS]);

#[derive(Default)]
pub struct Input {
    keys_now: Keys,
    keys_before: Keys,

    left_mouse_button: bool,
    left_mouse_toggled: bool,
    mouse: (i32, i32),
    mouse_in_previous_frame: (i32, i32),
    mouse_wheel: f32,
}

impl Default for Keys {
    fn default() -> Keys {
        let default = KeyboardInput {
            scancode: 0,
            state: ElementState::Released,
            virtual_keycode: None,
            modifiers: ModifiersState {
                shift: false,
                ctrl: false,
                alt: false,
                logo: false,
            },
        };
        Keys([default; NUM_KEYS])
    }
}

impl Input {
    pub fn prepare_for_next_frame(&mut self) {
        self.left_mouse_toggled = false;
        self.mouse_wheel = 0.0;
        self.mouse_in_previous_frame.0 = self.mouse.0;
        self.mouse_in_previous_frame.1 = self.mouse.1;
    }

    pub fn register_key(&mut self, input: &KeyboardInput) {
        match *input {
            KeyboardInput {
                virtual_keycode: Some(keycode),
                ..
            } => {
                let keycode = keycode as usize;
                self.keys_before.0[keycode] = self.keys_now.0[keycode];
                self.keys_now.0[keycode] = *input;
            }
            _ => {}
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
        let old = self.left_mouse_button;
        if let MouseButton::Left = button {
            self.left_mouse_button = match state {
                ElementState::Pressed => true,
                ElementState::Released => false,
            };
        }
        if old != self.left_mouse_button {
            self.left_mouse_toggled = true;
        }
    }

    // ---

    pub fn is_key_down(&self, keycode: VirtualKeyCode) -> bool {
        self.keys_now.0[keycode as usize].state == ElementState::Pressed
    }

    pub fn is_key_toggled(&self, keycode: VirtualKeyCode) -> bool {
        self.keys_before.0[keycode as usize].state != self.keys_now.0[keycode as usize].state
    }

    pub fn is_key_toggled_down(&self, keycode: VirtualKeyCode) -> bool {
        self.is_key_down(keycode) && self.is_key_toggled(keycode)
    }

    pub fn is_key_toggled_up(&self, keycode: VirtualKeyCode) -> bool {
        !self.is_key_down(keycode) && self.is_key_toggled(keycode)
    }

    pub fn get_mouse_pos(&self) -> (f32, f32) {
        (self.mouse.0 as f32, self.mouse.1 as f32)
    }

    pub fn is_left_mouse_button_down(&self) -> bool {
        self.left_mouse_button
    }
    pub fn is_left_mouse_button_toggled(&self) -> bool {
        self.left_mouse_toggled
    }

    pub fn get_mouse_moved(&self) -> (f32, f32) {
        (
            (self.mouse.0 - self.mouse_in_previous_frame.0) as f32,
            (self.mouse.1 - self.mouse_in_previous_frame.1) as f32,
        )
    }

    pub fn get_mouse_wheel(&self) -> f32 {
        self.mouse_wheel
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setting_ctrl_key() {
        let mut input = Input::default();
        assert_eq![false, input.get_ctrl()];
        input.set_ctrl();
        assert_eq![true, input.get_ctrl()];
        input.prepare_for_next_frame();
        assert_eq![false, input.get_ctrl()];
    }

    #[test]
    fn ensure_boundaries_ok() {
        let mut input = Input::default();
        input.register_key_down(VirtualKeyCode::Cut);
    }
}
