use super::*;
use cmdmat::Decider;
use std::net::TcpStream;

pub type GshSpawn = (
    JoinHandle<()>,
    Arc<AtomicBool>,
    mpsc::Receiver<Box<Fn(&mut Main) + Send>>,
);
pub type SomeDec = Option<&'static Decider<Input, GshDecision>>;
pub type Gsh<'a> = GameShell<Arc<cmdmat::Mapping<'a, Input, GshDecision, GameShellContext>>>;

pub struct GshTcp<'a, 'b> {
    pub gsh: &'a mut Gsh<'b>,
    pub stream: TcpStream,
    pub parser: PartialParse,
}

pub enum GshDecision {
    Help(String),
    Err(String),
}

#[derive(Clone)]
pub enum Input {
    Atom(String),
    Bool(bool),
    Command(String),
    F32(f32),
    I32(i32),
    String(String),
    U8(u8),
}
