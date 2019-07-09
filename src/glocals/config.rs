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
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
#[serde(default)]
struct Config {
    world: WorldConfig {
        gravity: f32,
        gravity_on: bool,
        air_fri_x: f32,
        air_fri_y: f32,
        ground_fri: f32,
        width: u32,
        height: u32,
        player: PlayerConfig {
            horizontal_acc: f32,
            jump_duration: f32,
            jump_acc: f32,
            acc: f32,
            max_vel: f32,
        }
    }
    controls: ControlsConfig {
        down: String,
        // up: String,
        // left: String,
        // right: String,
    }
    server: ServerConfig {
        srv_tick_ms: f32,
    }
    client: ClientConfig {
        snapshot_rate: f32,
        fps: f32,
    }
}

}
