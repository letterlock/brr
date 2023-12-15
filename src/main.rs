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
    // TODO: there are a bunch of index slices that this lint marks
    // that i need to check and either allow or handle differently
    clippy::indexing_slicing,
)]

mod die;
mod terminal;
mod editor;
mod document;
mod append_buffer;
mod row;

use die::die;
use terminal::Terminal;
use editor::{
    Editor,
    Position
};
use document::Document;
use append_buffer::AppendBuffer;
use row::{
    FileRow,
    DisplayRow,
};

use std::io::stdin;

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

// TODO:
//   - !!! config file
//   - !!! finish -h output
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
    let mut args = std::env::args();
    let mut user_input = String::new();
    let welcome_dialogue = 
"\r
welcome to\r
  ______                \r
  ___  /________________\r
  __  __ -_  ___/_  ___/\r
  _  /_/ /  /   _  /    \r
  /_.___//_/    /_/     \r
                        \r
    the perfunctory prose proliferator\r
\r
please specify a file name, type 'help' for help, or press ctrl+c to exit.";
    let help_dialogue = 
"brr help:\r
  -> usage: brr [OPTIONS/COMMANDS] [FILENAME]\r
  -h option or 'help' command prints this dialogue.";
    let error_dialogue = "
  -> usage: brr [OPTIONS/COMMANDS] [FILENAME]\r
  use option '-h' or command 'help' for help.";

    if let Some(arg) = args.nth(1) {
        match arg.as_str() {
            "-h"
            | "help" => println!("{help_dialogue}"),
            _ => Editor::default(&arg).run(),
        }
        
    } else {
        println!("{welcome_dialogue}");
        if let Err(error_msg) = stdin().read_line(&mut user_input) {
            println!("error: {error_msg}\r{error_dialogue}");
        } else if user_input.trim() == "help" {
            println!("{help_dialogue}\n\r  press ctrl+c to quit.");
            // this is hack-y but good enough for now
            while user_input.trim() != "todo" {};
        } else {
            Editor::default(user_input.trim()).run();
        };
    };
}