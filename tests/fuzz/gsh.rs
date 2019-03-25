use honggfuzz::fuzz;
use logger::Logger;
use metac::Evaluate;
use universe::mediators::game_shell::make_new_gameshell;

fn main() {
    let mut logger = Logger::spawn();
    logger.set_log_level(0);
    let mut gsh = make_new_gameshell(logger);
    loop {
        fuzz!(|data: &[u8]| {
            if let Ok(data) = std::str::from_utf8(data) {
                let _ = gsh.interpret_single(data);
                let _ = gsh.interpret_multiple(data);
            }
        });
    }
}
