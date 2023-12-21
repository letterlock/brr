use crate::Position;
use unicode_segmentation::UnicodeSegmentation;

// -----------------

#[derive(Default)]
pub struct AppendBuffer {
    pub buffer: String,
    pub word_count: usize,
    pub join_pos: Position,
}

impl AppendBuffer {
    pub fn insert(&mut self, char: char) {
        self.buffer.push(char);
    }

    // count words in the buffer
    pub fn count_words(&mut self) {
        self.word_count = self.buffer.unicode_words().count();
    }

    pub fn delete(&mut self) {
        self.buffer.pop();
    }

    pub fn is_dirty(&self) -> bool {
        !self.buffer.is_empty()
    }
}