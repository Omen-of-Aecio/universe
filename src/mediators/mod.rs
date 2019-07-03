pub mod does_line_collide_with_grid;
pub mod game_shell;
#[cfg(test)]
pub mod testtools;

/*
#[cfg(test)]
mod tests {
    use super::*;
    use crate::mediators::testtools::*;
    use crate::game::Client;
    use fast_logger::Logger;

    #[test]
    fn basic_setup_gsh() {
        let mut main = Client::new(Logger::spawn_void());
        spawn_gameshell(&mut main);
        assert![main.threads.game_shell_channel.is_some()];
        assert_eq!["6", gsh(&mut main, "+ 1 2 3")];
    }

    #[test]
    fn gsh_change_gravity() {
        let mut cli = Client::default();
        spawn_gameshell(&mut cli);
        assert_eq![
            "Set gravity value",
            gsh(&mut cli, "config gravity set y 1.23")
        ];
        cli.tick_logic();
        assert_eq![1.23, cli.logic.config.world.gravity];
    }

    #[test]
    fn gsh_change_gravity_synchronous() {
        let mut cli = Client::default();
        spawn_gameshell(&mut cli);
        assert_eq![
            "Set gravity value",
            gsh_synchronous(&mut cli, "config gravity set y 1.23", |cli| cli.tick_logic())
        ];
        assert_eq![1.23, cli.logic.config.world.gravity];
    }

    #[test]
    fn gsh_get_fps() {
        let mut cli = Client::default();
        spawn_gameshell(&mut cli);
        assert_eq![
            "0",
            gsh_synchronous(&mut cli, "config fps get", |cli| cli.tick_logic())
        ];

        gsh(&mut cli, "config fps set 1.23");
        cli.tick_logic();

        assert_eq![
            "1.23",
            gsh_synchronous(&mut cli, "config fps get", |cli| cli.tick_logic())
        ];
    }
}
*/
