use honggfuzz::fuzz;
use std::sync::{atomic::AtomicBool, Arc};
use universe::mediators::game_shell::*;
use universe::glocals::GameShell;
use universe::libs::metac::Evaluate;

fn main() {
    // given
    let (mut logger, _) = universe::libs::logger::Logger::spawn();
    logger.set_log_level(0);
    let keep_running = Arc::new(AtomicBool::new(true));
    let mut nest = Nest::new();
    for spell in SPEC {
        build_nest(&mut nest, spell.0, spell.1);
    }
    let mut gsh = GameShell {
        logger,
        keep_running,
        commands: Arc::new(nest),
    };
    loop {
        fuzz!(|data: &[u8]| {
            if data.len() < 1_000 {
                return;
            }
            if let Ok(data) = std::str::from_utf8(data) {
                let _ = gsh.interpret_single(data);
                let _ = gsh.interpret_multiple(data);
            }
        });
    }
}
