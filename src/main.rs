#![warn(
    clippy::all, 
    clippy::pedantic, 
    clippy::correctness,
    clippy::suspicious,
    clippy::complexity,
    clippy::perf,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::unwrap_in_result,
    clippy::indexing_slicing,
    clippy::string_slice,
)]

mod die;
mod config;
mod init;
mod terminal;
mod editor;
mod metadata;
mod document;
mod append_buffer;
mod row;

use die::die;
use config::Config;
use init::Init;
use terminal::Terminal;
use editor::{Editor, Position, SaveType};
use metadata::{Metadata, get_conf_or_log_path};
use document::{Document, render};
use append_buffer::AppendBuffer;
use row::DisplayRow;

use {
    log::{LevelFilter, error, warn, info},
    simple_logging::log_to_file,
};

// -----------------

// there's no official way to count words (and even counting 
// characters is more complex than you think) so brr's word
// and character counts should be used as guidelines
// also brr doesn't currently count words or characters in
// the append buffer

// RE: mouse events
// as far as i can tell, there's no easy way to capture mouse
// events without causing a whole bunch of unneccessary loops
// of brr's editor.rs->run() function, which i just don't like.
// it causes the cursor to flicker when the mouse is moved
// and i find that distracting and ugly. because it's not a
// feature that feels very important to brr's utility, i won't
// be implementing it at this time.
// IDEA: maybe i could try my hand at asyncronous handling of
// mouse events, so that they run separate from the program's
// main loop and only actually affect something when they would
// be needed to?

// BUG: singular words that are longer than the entire width of the terminal break
// the line wrapping and display. i'm not going to fix this right now because it's
// a fairly unreachable edge case, but 

// TODO:
//   - !!! clean up save type detection in editor.rs + document.rs
//   - !!! look into word detection code to see if i can't make it work more intuitively
//   - !!! fix error handling in editor.rs::refresh_screen()
//   -  !! apparently mac uses only \r for newlines, this will probably cause issues
//   -  !! fix truncation in message bar and status bar
//   -  !! add code comments for clarity
//   -   ! https://doc.rust-lang.org/stable/rust-by-example/fn/closures.html
// TODO: if the config file is openable and readable, but the individual
// options are mangled somehow, the user should be informed without having
// to open the log file.
// MAYBE:
//   -     don't wrap spaces along with words
//   -     add search function to viewing mode
//   -     scrollbar
//   -     line numbers
//   -     handle wide characters https://github.com/rhysd/kiro-editor
//   -     truncate absolute paths?

#[allow(clippy::unwrap_used)]
fn main() {
    let args = std::env::args().nth(1);

    if let Some(log_path) = get_conf_or_log_path(false) {
        log_to_file(&log_path, LevelFilter::Info).unwrap();
        
        info!("using log path: {}", log_path.display());
    } else {
        panic!("cannot find executable. do you have permission to access the folder containing brr?")
    };

    match Init::default().welcome(args) {
        Ok(()) => (),
        Err(error_msg) => error!("[init.rs -> main.rs]: {error_msg} - couldn't flush stdout."),
    };
}
