use config::{config, get_paths_recurse};
use serde_derive::{Deserialize, Serialize};

// enum Key { A, B, C};
// impl ConfigValue for Key {
// fn from_value(v: Value) -> Key {
// if let Value::String(s) = v {
// // ...
// }
// }
// }

config! {
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct Config {
    world: World {
        gravity: f32,
        gravity_on: bool,
        // air_fri: (f32, f32),
        ground_fri: f32,
        width: u32,
        height: u32,
    }
    player: Player {
        horizontal_acc: f32,
        jump_duration: f32,
        jump_acc: f32,
    }
    controls: Controls {
        down: String,
        // up: String,
        // left: String,
        // right: String,
    }
    // server {
        // srv_tick_duration: Duration,
    // }
    client: Client {
        snapshot_rate: f32,
        fps: f32,
    }
}

}
