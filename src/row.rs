// despite the repetition, i think this makes
// the code more readable overall
#[allow(clippy::module_name_repetitions)]
#[derive(Default)]
pub struct DisplayRow {
    pub content: String,
    pub len: usize,
    pub line_no: usize,
}


impl From<(String, usize, usize)> for DisplayRow {
    fn from((line, len, line_no): (String, usize, usize)) -> Self {
        DisplayRow {
            content: line,
            len,
            line_no,
        }
    }
}
