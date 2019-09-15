use winit::*;

const NUM_KEYS: usize = 161;
const NUM_MOUSE_BUTTONS: usize = 256 + 3;

// ---

struct Keys([KeyboardInput; NUM_KEYS]);

impl Default for Keys {
    fn default() -> Self {
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

// ---

struct MouseButtons([MouseInput; NUM_MOUSE_BUTTONS]);

impl Default for MouseButtons {
    fn default() -> Self {
        let default = MouseInput {
            state: ElementState::Released,
            modifiers: ModifiersState {
                shift: false,
                ctrl: false,
                alt: false,
                logo: false,
            },
        };
        MouseButtons([default; NUM_MOUSE_BUTTONS])
    }
}

// ---

#[derive(Clone, Copy)]
pub struct MouseInput {
    pub state: ElementState,
    pub modifiers: ModifiersState,
}

#[derive(Default)]
pub struct Input {
    keys_now: Keys,
    keys_before: Keys,

    mouse_buttons_now: MouseButtons,
    mouse_buttons_before: MouseButtons,

    mouse: (i32, i32),
    mouse_in_previous_frame: (i32, i32),
    mouse_wheel: f32,
}

impl Input {
    pub fn prepare_for_next_frame(&mut self) {
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

    pub fn register_mouse_input(&mut self, state: MouseInput, button: MouseButton) {
        let index = mouse_button_to_index(button);
        self.mouse_buttons_before.0[index] = self.mouse_buttons_now.0[index];
        self.mouse_buttons_now.0[index] = state;
    }

    pub fn position_mouse(&mut self, x: i32, y: i32) {
        self.mouse.0 = x;
        self.mouse.1 = y;
    }

    pub fn register_mouse_wheel(&mut self, y: f32) {
        self.mouse_wheel = y;
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

    pub fn is_mouse_button_down(&self, button: MouseButton) -> bool {
        let index = mouse_button_to_index(button);
        self.mouse_buttons_now.0[index].state == ElementState::Pressed
    }

    pub fn is_mouse_button_toggled(&self, button: MouseButton) -> bool {
        let index = mouse_button_to_index(button);
        self.mouse_buttons_before.0[index].state != self.mouse_buttons_now.0[index].state
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

fn mouse_button_to_index(button: MouseButton) -> usize {
    match button {
        MouseButton::Left => 0,
        MouseButton::Right => 1,
        MouseButton::Middle => 2,
        MouseButton::Other(value) => 3 + value as usize,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tri_state_switch_pressed_released_pressed() {
        let mut input = Input::default();

        input.register_key(&KeyboardInput {
            scancode: 0,
            state: ElementState::Pressed,
            virtual_keycode: Some(VirtualKeyCode::A),
            modifiers: ModifiersState::default(),
        });

        input.register_key(&KeyboardInput {
            scancode: 0,
            state: ElementState::Released,
            virtual_keycode: Some(VirtualKeyCode::A),
            modifiers: ModifiersState::default(),
        });

        input.register_key(&KeyboardInput {
            scancode: 0,
            state: ElementState::Pressed,
            virtual_keycode: Some(VirtualKeyCode::A),
            modifiers: ModifiersState::default(),
        });

        assert_eq![true, input.is_key_toggled_down(VirtualKeyCode::A)];
        assert_eq![false, input.is_key_toggled_up(VirtualKeyCode::A)];
        assert_eq![true, input.is_key_down(VirtualKeyCode::A)];
    }

    #[test]
    fn tri_state_switch_released_pressed_released() {
        let mut input = Input::default();

        input.register_key(&KeyboardInput {
            scancode: 0,
            state: ElementState::Released,
            virtual_keycode: Some(VirtualKeyCode::A),
            modifiers: ModifiersState::default(),
        });

        input.register_key(&KeyboardInput {
            scancode: 0,
            state: ElementState::Pressed,
            virtual_keycode: Some(VirtualKeyCode::A),
            modifiers: ModifiersState::default(),
        });

        input.register_key(&KeyboardInput {
            scancode: 0,
            state: ElementState::Released,
            virtual_keycode: Some(VirtualKeyCode::A),
            modifiers: ModifiersState::default(),
        });

        assert_eq![false, input.is_key_toggled_down(VirtualKeyCode::A)];
        assert_eq![true, input.is_key_toggled_up(VirtualKeyCode::A)];
        assert_eq![false, input.is_key_down(VirtualKeyCode::A)];
    }
    #[test]
    fn ensure_boundaries_ok() {
        let mut input = Input::default();
        input.register_key(&KeyboardInput {
            scancode: 0,
            state: ElementState::Pressed,
            virtual_keycode: Some(VirtualKeyCode::Cut),
            modifiers: ModifiersState::default(),
        });

        input.register_key(&KeyboardInput {
            scancode: 0,
            state: ElementState::Pressed,
            virtual_keycode: None,
            modifiers: ModifiersState::default(),
        });
    }
}
