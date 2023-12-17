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

use die::die;
use terminal::Terminal;
use editor::{
    Editor,
    Position
};
use file::File;
use document::Document;
use append_buffer::AppendBuffer;
use row::{
    FileRow,
    DisplayRow,
};
use init::Init;

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

// possible config options:
// - don't check for files with .md/.txt when opening
// - quit times
// - start mode
// - 

// TODO:
//   - !!! test brr with absolute file paths
//   - !!! optimise document.rs
//   - !!! fix status bar and message bar so i like them more
//   - !!! config file
//   - !!! finish -h output
//   - !!! overuse of self. ?
//   - !!! log errors to file instead of panic
//   - !!! total amount written printed in goodbye message
//   - !!! clean code
//   - !!! check sizes (usize, u16) to avoid overflow
//   - !!! verify error handling works
//   -  !! add code comments for clarity
//   -  !! check if len() is inefficient
//   -   ! don't wrap spaces along with words
//   -   ! add search function to viewing mode
//   -   ! scrollbar
//   -   ! mouse scrolling in view mode
//   -   ! line numbers
//   -   ! handle wide characters

fn main() {
    let args = std::env::args().nth(1);

    Init::default().welcome(args);
}