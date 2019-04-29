fn main() {
    #[cfg(target_os = "linux")]
    {
        match std::os::unix::fs::symlink(".pre-commit", ".git/hooks/pre-commit") {
            Ok(()) => {}
            Err(ioerr) => {
                if ioerr.kind() != std::io::ErrorKind::AlreadyExists {
                    panic!["Unable to create link: {:?}", ioerr];
                }
            }
        }
    }
}
