// despite the repetition, i think this makes
// the code more readable overall
#[allow(clippy::module_name_repetitions)]
#[derive(Default)]
pub struct DisplayRow {
    pub content: String,
    pub len: usize,
    pub is_buffer: bool,
}
