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
)]

mod die;
mod terminal;
mod editor;
mod file;
mod document;
mod append_buffer;
mod row;
mod init;
mod config;

use die::die;
use log::error;
use terminal::Terminal;
use editor::{
    Editor,
    Position
};
use file::{File, get_conf_or_log_path};
use document::Document;
use document::render;
use append_buffer::AppendBuffer;
use row::{
    FileRow,
    DisplayRow,
};
use init::Init;
use config::Config;

use log::{LevelFilter, warn, trace};

// note on converting usize to u16:
// in reality usize is unneccesary for brr because a document is
// very unlikely to pass 65536 lines (the maximum value of u16)
// by my calculations, to surpass 65k lines, a document would
// have to be (conservatively) over 2000 a4 pages long in standard 
// manuscript format: 
//   https://en.wikipedia.org/wiki/Standard_manuscript_format
// likewise -- a single line would have to be about 40 pages long
// before it was truncated due to being longer than 65536 chars
// calculations:
//   page size:      a4
//   font size:      12pt monospaced
//   line spacing:   double
//   chars per line: 65
//   lines per page: 25
// max lines in document as pages:
//   65536 / 25 = 2621.44
// max chars in line as pages:
//   65 * 25 = 1625
//   65536 / 1625 = 40.33

// brr saves - after 5 seconds of inactivity or every five
// words (as you finish the sixth word)

// there's no official way to count words (and even counting 
// characters is more complex than you think) so brr's word
// and character counts should be used as guidelines
// also brr doesn't currently count words or characters in
// the append buffer

// NEXT: figure out packaging and distribution.

// TODO:
//   - !!! optimise document.rs https://doc.rust-lang.org/stable/rust-by-example/std_misc/file/read_lines.html
//   - !!! avoid overuse of .clone()
//   - !!! check sizes (usize, u16) to avoid overflow
//   - !!! overuse of self. ?
//   - !!! tidy up code
//   - !!! fix errors in editor.rs::refresh_screen()
//   -  !! config file description
//   -  !! add code comments for clarity
// MAYBE:
//   -     don't wrap spaces along with words
//   -     add search function to viewing mode
//   -     scrollbar
//   -     mouse scrolling in view mode
//   -     line numbers
//   -     handle wide characters https://github.com/rhysd/kiro-editor
//   -     truncate absolute paths?

#[allow(clippy::unwrap_used)]
fn main() {
    let args = std::env::args().nth(1);

    if let Some(log_path) = get_conf_or_log_path(false) {
        simple_logging::log_to_file(&log_path, LevelFilter::Trace).unwrap();
        
        if let Some(path_string) = log_path.to_str() {
            trace!("[main.rs]: using log path: {path_string}");
        }
    } else {
        panic!("cannot find executable. do you have permission to access the folder containing brr?")
    };

    match Init::default().welcome(args) {
        Ok(()) => (),
        Err(error_msg) => error!("[init.rs -> main.rs]: {error_msg} - couldn't flush stdout."),
    };
}