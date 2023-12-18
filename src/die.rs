use std::io::Error;
use std::io::stdout;

use crossterm::{terminal::{
    disable_raw_mode,
    LeaveAlternateScreen,
    },
    ExecutableCommand,
};

#[allow(clippy::needless_pass_by_value)]
pub fn die(error_msg: Error) {
    if let Err(error_msg) = disable_raw_mode() {
        println!("could not disable raw mode: {error_msg}");
    };
    if let Err(error_msg) = stdout().execute(LeaveAlternateScreen) {
        println!("could not leave alternate screen: {error_msg}");
    };
    panic!("{error_msg}");
}
