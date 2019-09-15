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

    mouse_now: (i32, i32),
    mouse_before: (i32, i32),

    mouse_wheel: f32,
}

impl Input {
    pub fn prepare_for_next_frame(&mut self) {
        self.mouse_wheel = 0.0;
        self.mouse_before.0 = self.mouse_now.0;
        self.mouse_before.1 = self.mouse_now.1;
    }

    // ---

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

    pub fn is_key_down(&self, keycode: VirtualKeyCode) -> bool {
        self.keys_now.0[keycode as usize].state == ElementState::Pressed
    }

    pub fn is_key_up(&self, keycode: VirtualKeyCode) -> bool {
        self.keys_now.0[keycode as usize].state == ElementState::Released
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

    pub fn key_modifiers_state(&self, keycode: VirtualKeyCode) -> ModifiersState {
        self.keys_now.0[keycode as usize].modifiers
    }

    // ---

    pub fn register_mouse_input(&mut self, state: MouseInput, button: MouseButton) {
        let index = mouse_button_to_index(button);
        self.mouse_buttons_before.0[index] = self.mouse_buttons_now.0[index];
        self.mouse_buttons_now.0[index] = state;
    }

    pub fn is_mouse_button_down(&self, button: MouseButton) -> bool {
        let index = mouse_button_to_index(button);
        self.mouse_buttons_now.0[index].state == ElementState::Pressed
    }

    pub fn is_mouse_button_up(&self, button: MouseButton) -> bool {
        let index = mouse_button_to_index(button);
        self.mouse_buttons_now.0[index].state == ElementState::Released
    }

    pub fn is_mouse_button_toggled(&self, button: MouseButton) -> bool {
        let index = mouse_button_to_index(button);
        self.mouse_buttons_before.0[index].state != self.mouse_buttons_now.0[index].state
    }

    pub fn is_mouse_button_toggled_down(&self, button: MouseButton) -> bool {
        self.is_mouse_button_toggled(button) && self.is_mouse_button_down(button)
    }

    pub fn is_mouse_button_toggled_up(&self, button: MouseButton) -> bool {
        self.is_mouse_button_toggled(button) && self.is_mouse_button_up(button)
    }

    pub fn mouse_button_modifiers_state(&self, button: MouseButton) -> ModifiersState {
        let index = mouse_button_to_index(button);
        self.mouse_buttons_now.0[index].modifiers
    }

    // ---

    pub fn register_mouse_position(&mut self, x: i32, y: i32) {
        self.mouse_now.0 = x;
        self.mouse_now.1 = y;
    }

    pub fn register_mouse_wheel(&mut self, y: f32) {
        self.mouse_wheel += y;
    }

    pub fn get_mouse_position(&self) -> (f32, f32) {
        (self.mouse_now.0 as f32, self.mouse_now.1 as f32)
    }

    pub fn get_mouse_moved(&self) -> (f32, f32) {
        (
            (self.mouse_now.0 - self.mouse_before.0) as f32,
            (self.mouse_now.1 - self.mouse_before.1) as f32,
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
    fn tri_state_modifiers() {
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
            modifiers: ModifiersState {
                ctrl: true,
                ..ModifiersState::default()
            },
        });

        input.register_key(&KeyboardInput {
            scancode: 0,
            state: ElementState::Pressed,
            virtual_keycode: Some(VirtualKeyCode::A),
            modifiers: ModifiersState {
                shift: true,
                ..ModifiersState::default()
            },
        });

        assert_eq![true, input.is_key_toggled_down(VirtualKeyCode::A)];
        assert_eq![false, input.is_key_toggled_up(VirtualKeyCode::A)];
        assert_eq![true, input.is_key_down(VirtualKeyCode::A)];
        assert_eq![false, input.key_modifiers_state(VirtualKeyCode::A).ctrl];
        assert_eq![true, input.key_modifiers_state(VirtualKeyCode::A).shift];
    }

    #[test]
    fn tri_state_mouse_input() {
        let mut input = Input::default();

        input.register_mouse_input(
            MouseInput {
                state: ElementState::Pressed,
                modifiers: ModifiersState::default(),
            },
            MouseButton::Left,
        );

        input.register_mouse_input(
            MouseInput {
                state: ElementState::Released,
                modifiers: ModifiersState::default(),
            },
            MouseButton::Left,
        );

        input.register_mouse_input(
            MouseInput {
                state: ElementState::Pressed,
                modifiers: ModifiersState::default(),
            },
            MouseButton::Left,
        );

        assert_eq![true, input.is_mouse_button_toggled(MouseButton::Left)];
        assert_eq![true, input.is_mouse_button_down(MouseButton::Left)];
        assert_eq![false, input.is_mouse_button_up(MouseButton::Left)];
        assert_eq![true, input.is_mouse_button_toggled_down(MouseButton::Left)];
        assert_eq![false, input.is_mouse_button_toggled_up(MouseButton::Left)];
        assert_eq![
            ModifiersState::default(),
            input.mouse_button_modifiers_state(MouseButton::Left)
        ];
    }

    #[test]
    fn tri_state_mouse_modifiers() {
        let mut input = Input::default();

        input.register_mouse_input(
            MouseInput {
                state: ElementState::Pressed,
                modifiers: ModifiersState::default(),
            },
            MouseButton::Left,
        );

        input.register_mouse_input(
            MouseInput {
                state: ElementState::Released,
                modifiers: ModifiersState {
                    alt: true,
                    ..ModifiersState::default()
                },
            },
            MouseButton::Left,
        );

        input.register_mouse_input(
            MouseInput {
                state: ElementState::Pressed,
                modifiers: ModifiersState {
                    logo: true,
                    ..ModifiersState::default()
                },
            },
            MouseButton::Left,
        );

        assert_eq![
            true,
            input.mouse_button_modifiers_state(MouseButton::Left).logo
        ];
        assert_eq![
            false,
            input.mouse_button_modifiers_state(MouseButton::Left).alt
        ];
    }

    #[test]
    fn only_consider_last_mouse_pos() {
        let mut input = Input::default();
        input.register_mouse_position(1, 1);
        input.register_mouse_position(8, 9);
        input.register_mouse_position(123, 456);
        input.register_mouse_position(3, 6);

        assert_eq![(3.0, 6.0), input.get_mouse_position()];
        assert_eq![(3.0, 6.0), input.get_mouse_moved()];

        input.prepare_for_next_frame();

        assert_eq![(3.0, 6.0), input.get_mouse_position()];
        assert_eq![(0.0, 0.0), input.get_mouse_moved()];
    }

    #[test]
    fn accumulate_mouse_wheel_deltas() {
        let mut input = Input::default();
        input.register_mouse_wheel(0.1);
        input.register_mouse_wheel(0.8);
        input.register_mouse_wheel(0.3);
        assert_eq![1.2, input.get_mouse_wheel()];

        input.prepare_for_next_frame();

        assert_eq![0.0, input.get_mouse_wheel()];
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

        input.register_mouse_input(
            MouseInput {
                state: ElementState::Pressed,
                modifiers: ModifiersState::default(),
            },
            MouseButton::Other(255),
        );
    }
}
