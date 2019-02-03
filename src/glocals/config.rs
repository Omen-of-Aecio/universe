#[macro_use]
use config::config;
// enum Key { A, B, C};
// impl ConfigValue for Key {
    // fn from_value(v: Value) -> Key {
        // if let Value::String(s) = v {
            // // ...
        // }
    // }
// }

config! {
    physics: Physics {
        gravity: f32,
    }
    controls: Controls {
        down: String, // -> Key (TODO)
    }
}

// Turns into:
/*
struct Physics {
    pub gravity: f32,
}
struct Controls {
    pub down: Key,
}
struct Config {
    pub physics: Physics,
    pub controls: Controls,
}
impl Config {
    pub fn update(&mut self, name: String, value: Value) {
        if name == "physics gravity" {
            self.physics.gravity = value.to_num(),
        } else if name == "controls down" {
            self.controls.down = key::from_value(value),
        }
    }
}
*/
