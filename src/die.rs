use {
    std::io::{Error, stdout},
    crossterm::{
        cursor::SetCursorStyle,
        terminal::{disable_raw_mode, LeaveAlternateScreen},
        ExecutableCommand,
    },
    log::error,
};

// -----------------

#[allow(clippy::needless_pass_by_value)]
pub fn die(error_msg: Error) {
    if let Err(error_msg) = stdout().execute(SetCursorStyle::DefaultUserShape) {
        error!("[die.rs]: {error_msg} - could not reset cursor style.");
    };
    if let Err(error_msg) = disable_raw_mode() {
        error!("[die.rs]: {error_msg} - could not disable raw mode.");
    };
    if let Err(error_msg) = stdout().execute(LeaveAlternateScreen) {
        error!("[die.rs]: {error_msg} - could not leave alternate screen.");
    };
    panic!("{error_msg}");
}
