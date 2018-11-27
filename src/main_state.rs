use clap;
use conf;
use slog::Logger;

pub struct Main<'a> {
    pub _logger_guard: slog_scope::GlobalLoggerGuard,
    pub config: Option<conf::Config>,
    pub look: u32,
    pub options: clap::ArgMatches<'a>,
}
