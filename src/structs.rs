pub struct Suggestion {
    pub text: String,
    pub match_indices: Vec<usize>,
    pub score: usize,
}

pub mod terminal_guard {
    use crossterm::terminal;
    use std::io;

    pub struct TerminalGuard;

    impl TerminalGuard {
        pub fn new() -> io::Result<Self> {
            terminal::enable_raw_mode()?;
            Ok(Self)
        }
    }

    impl Drop for TerminalGuard {
        fn drop(&mut self) {
            let _ = terminal::disable_raw_mode();
        }
    }
}
