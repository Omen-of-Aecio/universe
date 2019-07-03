use crate::game::Client;
use std::net::TcpStream;
use std::{
    sync::{atomic::AtomicBool, mpsc, Arc},
    thread::JoinHandle,
};

pub mod config;
pub mod log;
pub mod msg;

pub use self::config::Config;
pub use log::Log;
pub use msg::*;

pub type GshChannelRecv = mpsc::Receiver<Box<dyn Fn(&mut Client) + Send>>;
pub type GshChannelSend = mpsc::SyncSender<Box<dyn Fn(&mut Client) + Send>>;

pub struct GshSpawn {
    pub thread_handle: JoinHandle<()>,
    pub keep_running: Arc<AtomicBool>,
    pub channel: mpsc::Receiver<Box<dyn Fn(&mut Client) + Send>>,
    pub port: u16,
    pub channel_send: mpsc::SyncSender<Box<dyn Fn(&mut Client) + Send>>,
}

#[derive(Default)]
pub struct Threads {
    // TODO name
    pub game_shell: Option<std::thread::JoinHandle<()>>,
    pub game_shell_keep_running: Option<Arc<AtomicBool>>,
    pub game_shell_port: Option<u16>,
    pub game_shell_channel: Option<GshChannelRecv>,
    pub game_shell_channel_send: Option<mpsc::SyncSender<Box<dyn Fn(&mut Client) + Send>>>,
    pub game_shell_connection: Option<TcpStream>,
}
