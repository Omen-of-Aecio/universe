use crate::glocals::*;

pub fn entry_point_server(s: &mut Main) {
    loop {
        s.time = std::time::Instant::now();
        server_tick(s);
        if s.server.is_none() {
            break;
        }
    }
}

fn server_tick(_s: &mut Main) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_server_if_none() {
        let mut main = Main::default();
        entry_point_server(&mut main);
    }
}
