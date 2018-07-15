//! Structs in this module are only used for deserialization of toml file, not for e.g.
//! representation runtime.

use toml;
use std::fs::File;
use std::io::Read;
use err::*;


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
    pub gravity: f32,
}
#[derive(Deserialize, Clone)]
pub struct WorldConfig {
    pub width: u32,
    pub height: u32,
}
#[derive(Deserialize, Clone)]
pub struct ServerConfig {
    pub fps: u32,
}

impl Config {
    pub fn from_file(path: &str) -> Result<Config> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        Config::from_str(&contents)
    }
    pub fn from_str(s: &str) -> Result<Config> {
        Ok(toml::from_str(s)?)
    }

    pub fn get_srv_frame_duration(&self) -> f32 {
        (1.0 / self.srv.fps as f32)
    }
}
