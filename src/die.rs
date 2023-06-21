use crate::Terminal;
use crossterm::terminal::disable_raw_mode;

#[allow(clippy::expect_used)]
pub fn die(error_msg: &std::io::Error) {
    Terminal::clear_screen();
    disable_raw_mode().expect("could not disable raw mode");
    panic!("{error_msg}");
}