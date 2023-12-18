use std::io::Error;
use std::io::stdout;

use crossterm::{terminal::{
    disable_raw_mode,
    LeaveAlternateScreen,
    },
    ExecutableCommand,
};
use log::error;

#[allow(clippy::needless_pass_by_value)]
pub fn die(error_msg: Error) {
    if let Err(error_msg) = disable_raw_mode() {
        error!("[die.rs]: {error_msg} - could not disable raw mode");
    };
    if let Err(error_msg) = stdout().execute(LeaveAlternateScreen) {
        error!("[die.rs]: {error_msg} - could not leave alternate screen");
    };
    panic!("{error_msg}");
}
