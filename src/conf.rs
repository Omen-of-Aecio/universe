//! Structs in this module are only used for deserialization of toml file, not for e.g.
//! representation runtime.

use err::*;
use std::fs::File;
use std::io::Read;
use std::time::Duration;
use toml;

#[derive(Deserialize, Clone)]
pub struct Config {
    pub player: PlayerConfig,
    pub world: WorldConfig,
    pub srv: ServerConfig,
}

#[derive(Deserialize, Clone)]
pub struct PlayerConfig {
    pub hori_acc: f32,
    pub jump_duration: f32,
    pub jump_delay: f32,
    pub jump_acc: f32,
    // Client config
    // pub snapshot_rate: f32,
}
#[derive(Deserialize, Clone)]
pub struct WorldConfig {
    pub width: u32,
    pub height: u32,
    pub gravity: f32,
    pub air_fri: (f32, f32),
    pub ground_fri: f32,
}

#[derive(Deserialize, Clone)]
pub struct ServerConfig {
    /// Ticks per second
    pub tps: u32,
}

impl Config {
    pub fn from_file(path: &str) -> Result<Config, Error> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        Config::from_str(&contents)
    }
    pub fn from_str(s: &str) -> Result<Config, Error> {
        Ok(toml::from_str(s)?)
    }

    pub fn get_srv_tick_duration(&self) -> Duration {
        Duration::from_nanos((1_000_000_000.0 / self.srv.tps as f32) as u64)
    }
}
