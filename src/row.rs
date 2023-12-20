use unicode_segmentation::UnicodeSegmentation;

// despite the repetition, i think this makes
// the code more readable overall
#[allow(clippy::module_name_repetitions)]
#[derive(Default)]

pub struct FileRow {
    pub content: String,
    pub len: usize,
}

// despite the repetition, i think this makes
// the code more readable overall
#[allow(clippy::module_name_repetitions)]
#[derive(Default)]
pub struct DisplayRow {
    pub content: String,
    pub len: usize,
    pub is_buffer: bool,
}

impl From<&str> for FileRow {
    fn from(slice: &str) -> Self {
        Self {
            content: String::from(slice),
            len: slice.graphemes(true).count(),
        }
    }
}

// impl FileRow {
//     pub fn as_bytes(&self) -> &[u8] {
//         self.content.as_bytes()
//     }
// }

// impl DisplayRow {
//     pub fn render(&self) -> String {
//         let mut line = String::new();
        
//         for grapheme in self.content[..]
//         .graphemes(true) {
//             if grapheme == "\t" {
//                 line.push_str("  ");
//             } else {
//                 line.push_str(grapheme);
//             }
//         }

//         line
//     }
// }
