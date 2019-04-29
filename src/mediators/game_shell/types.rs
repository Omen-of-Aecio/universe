use super::*;
use cmdmat::Decider;
use std::net::TcpStream;

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
