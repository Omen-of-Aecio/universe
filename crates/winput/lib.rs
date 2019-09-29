//! Input event accumulator for [winit].
//!
//! Used to accumulate events and distribute them throughout an application. This implementation
//! uses a 2-window for keyboard and mouse buttons so it can capture an inter-frame toggle while
//! enforcing a single action per frame.
//! ```
//! use winit::*;
//! use winput::Input;
//!
//! let mut input = Input::default();
//!
//! input.register_key(&KeyboardInput {
//!     scancode: 0,
//!     state: ElementState::Pressed,
//!     virtual_keycode: Some(VirtualKeyCode::A),
//!     modifiers: ModifiersState::default(),
//! });
//!
//! assert![input.is_key_toggled_down(VirtualKeyCode::A)];
//!
//! input.register_mouse_position(1f32, 2f32);
//! ```
#![deny(
    missing_copy_implementations,
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]
use std::fmt;
use winit::*;

const NUM_KEYS: usize = 161;
const NUM_MOUSE_BUTTONS: usize = 256 + 3;

// ---

struct Keys([KeyInput; NUM_KEYS]);

impl fmt::Debug for Keys {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for idx in 0..self.0.len() - 1 {
            write![f, "{:?}", self.0[idx]]?;
        }
        write![f, "{:?}", self.0.last()]
    }
}

impl Default for Keys {
    fn default() -> Self {
        let default = KeyInput {
            state: ElementState::Released,
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

impl fmt::Debug for MouseButtons {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for idx in 0..self.0.len() - 1 {
            write![f, "{:?}", self.0[idx]]?;
        }
        write![f, "{:?}", self.0.last()]
    }
}

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

/// Position of the mouse
#[derive(Clone, Copy)]
pub struct MousePosition(i32, i32);

/// Keyboard input as a buttonstate and modifier state
#[derive(Clone, Copy, Debug)]
pub struct KeyInput {
    /// Modifiers pressed while this event occurred
    pub modifiers: ModifiersState,
    /// State of the button
    pub state: ElementState,
}

/// Mouse input as a buttonstate and a modifier state
#[derive(Clone, Copy, Debug)]
pub struct MouseInput {
    /// State of the button
    pub state: ElementState,
    /// Modifiers pressed while this event occurred
    pub modifiers: ModifiersState,
}

// ---

impl From<KeyboardInput> for KeyInput {
    fn from(input: KeyboardInput) -> Self {
        KeyInput {
            state: input.state,
            modifiers: input.modifiers,
        }
    }
}

/// 2-window for accumulating [winit] input events.
///
/// This struct accumulates input events and allows them to be used throughout the program. Its
/// main purpose is to resolve issues of multiple keypresses per-frame as well as accumulating
/// mouse events such as position and mousewheel events.
#[derive(Debug, Default)]
pub struct Input {
    keys_now: Keys,
    keys_before: Keys,

    mouse_buttons_now: MouseButtons,
    mouse_buttons_before: MouseButtons,

    mouse_now: (f32, f32),
    mouse_before: (f32, f32),

    mouse_wheel: f32,
}

impl Input {
    /// Clear delta-based inputs such as mouse-wheel, and overwrite the previous mouse position
    pub fn prepare_for_next_frame(&mut self) {
        self.mouse_wheel = 0.0;
        self.mouse_before.0 = self.mouse_now.0;
        self.mouse_before.1 = self.mouse_now.1;
    }

    // ---

    /// Register a keyboard input
    pub fn register_key(&mut self, input: &KeyboardInput) {
        if let KeyboardInput {
            virtual_keycode: Some(keycode),
            ..
        } = input
        {
            let keycode = *keycode as usize;
            self.keys_before.0[keycode] = self.keys_now.0[keycode];
            self.keys_now.0[keycode] = KeyInput::from(*input);
        }
    }

    /// Check if a key is pressed
    pub fn is_key_down(&self, keycode: VirtualKeyCode) -> bool {
        self.keys_now.0[keycode as usize].state == ElementState::Pressed
    }

    /// Check if a key is up (released)
    pub fn is_key_up(&self, keycode: VirtualKeyCode) -> bool {
        self.keys_now.0[keycode as usize].state == ElementState::Released
    }

    /// Check if a key has been toggled
    pub fn is_key_toggled(&self, keycode: VirtualKeyCode) -> bool {
        self.keys_before.0[keycode as usize].state != self.keys_now.0[keycode as usize].state
    }

    /// Check if a key has been toggled and is pressed
    pub fn is_key_toggled_down(&self, keycode: VirtualKeyCode) -> bool {
        self.is_key_down(keycode) && self.is_key_toggled(keycode)
    }

    /// Check if a key has been toggled and is released
    pub fn is_key_toggled_up(&self, keycode: VirtualKeyCode) -> bool {
        !self.is_key_down(keycode) && self.is_key_toggled(keycode)
    }

    /// Get a key's modifiers state
    pub fn key_modifiers_state(&self, keycode: VirtualKeyCode) -> ModifiersState {
        self.keys_now.0[keycode as usize].modifiers
    }

    // ---

    /// Register a mouse button event
    pub fn register_mouse_input(&mut self, state: MouseInput, button: MouseButton) {
        let index = mouse_button_to_index(button);
        self.mouse_buttons_before.0[index] = self.mouse_buttons_now.0[index];
        self.mouse_buttons_now.0[index] = state;
    }

    /// Check if a mouse button is pressed
    pub fn is_mouse_button_down(&self, button: MouseButton) -> bool {
        let index = mouse_button_to_index(button);
        self.mouse_buttons_now.0[index].state == ElementState::Pressed
    }

    /// Check if a mouse button is released (up)
    pub fn is_mouse_button_up(&self, button: MouseButton) -> bool {
        let index = mouse_button_to_index(button);
        self.mouse_buttons_now.0[index].state == ElementState::Released
    }

    /// Check if a mouse button is toggled
    pub fn is_mouse_button_toggled(&self, button: MouseButton) -> bool {
        let index = mouse_button_to_index(button);
        self.mouse_buttons_before.0[index].state != self.mouse_buttons_now.0[index].state
    }

    /// Check if a mouse button is toggled and is pressed
    pub fn is_mouse_button_toggled_down(&self, button: MouseButton) -> bool {
        self.is_mouse_button_toggled(button) && self.is_mouse_button_down(button)
    }

    /// Check if a mouse button is toggled and is released
    pub fn is_mouse_button_toggled_up(&self, button: MouseButton) -> bool {
        self.is_mouse_button_toggled(button) && self.is_mouse_button_up(button)
    }

    /// Get a mouse button's modifiers state
    pub fn mouse_button_modifiers_state(&self, button: MouseButton) -> ModifiersState {
        let index = mouse_button_to_index(button);
        self.mouse_buttons_now.0[index].modifiers
    }

    // ---

    /// Register the position of the mouse
    pub fn register_mouse_position(&mut self, x: f32, y: f32) {
        self.mouse_now.0 = x;
        self.mouse_now.1 = y;
    }

    /// Register a scroll wheel event
    pub fn register_mouse_wheel(&mut self, y: f32) {
        self.mouse_wheel += y;
    }

    /// Get the current mouse position
    pub fn get_mouse_position(&self) -> (f32, f32) {
        (self.mouse_now.0, self.mouse_now.1)
    }

    /// Get the mouse movement since last frame
    pub fn get_mouse_moved(&self) -> (f32, f32) {
        (
            (self.mouse_now.0 - self.mouse_before.0),
            (self.mouse_now.1 - self.mouse_before.1),
        )
    }

    /// Get the current mouse wheel value
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
        input.register_mouse_position(1f32, 1f32);
        input.register_mouse_position(8f32, 9f32);
        input.register_mouse_position(123f32, 456f32);
        input.register_mouse_position(3f32, 6f32);

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
