use super::*;
use crate::glocals::GshChannelSend;
use cmdmat::Decider;
use std::net::TcpStream;

#[derive(Clone)]
pub struct GameShellContext {
    pub config_change: Option<GshChannelSend>,
    pub logger: Logger<Log>,
    pub keep_running: Arc<AtomicBool>,
    pub variables: HashMap<String, String>,
}
