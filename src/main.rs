#![warn(
    clippy::all, 
    clippy::pedantic, 
    clippy::correctness,
    clippy::suspicious,
    clippy::complexity,
    clippy::perf,
    //clippy::expect_used,
    //clippy::unwrap_used,
    //clippy::unwrap_in_result,
    //clippy::question_mark_used,
    //clippy::string_slice,
    //clippy::indexing_slicing,
)]
mod document;
mod row;
mod editor;
mod terminal;
mod die;

// just in case this breaks something later:
// removed 'pub' keyword from terminal, editor,
// document, and row below. didn't seem to matter
use editor::Editor;
use terminal::Terminal;
use editor::{ CursorPosition, SearchDirection };
use document::Document;
use row::Row;
use die::die;

// todo: handle errors instead of using .ok()
// todo: line numbers
// todo: line wrapping
// todo: scroll editor when out of bounds of single
//       line instead of terminal screen
// todo: configuration file
// todo?: search currently doesn't loop
// todo?: hightlighting for markdown?

fn main() {
    Editor::default().run();
}
