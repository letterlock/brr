#![warn(
    clippy::all, 
    clippy::pedantic, 
    clippy::correctness,
    clippy::suspicious,
    clippy::complexity,
    clippy::perf,
    //clippy::expect_used,
    //clippy::unwrap_used,
    clippy::unwrap_in_result,
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
use editor::CursorPosition;
use document::Document;
use row::Row;
use die::die;

// todo: handle errors instead of using .ok()

fn main() {
    Editor::default().run();
}
