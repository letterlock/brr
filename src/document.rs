use crate::Terminal;
use crate::File;
use crate::FileRow;
use crate::DisplayRow;
use crate::AppendBuffer;

use log::trace;
use unicode_segmentation::UnicodeSegmentation;
use std::{
    time::Instant,
    io::{
        Error,
        Write,
    },
    fs::rename,
};

// #[derive(Default)]
pub struct Document {
    pub file: File,
    pub file_rows: Vec<FileRow>,
    pub display_rows: Vec<DisplayRow>,
    pub append_buffer: AppendBuffer,
    pub last_edit: Instant,
    pub count: Count,
    pub start_count: Count,
}

#[derive(Default, Clone)]
pub struct Count {
    pub words: usize,
    pub chars: usize,
}

impl Document {
    pub fn create(file: File) -> Self {
        let file_rows = vec![FileRow::default()];

        Self { 
            file,
            file_rows,
            display_rows: Vec::new(),
            append_buffer: AppendBuffer::default(),
            last_edit: Instant::now(),
            count: Count::default(),
            start_count: Count::default(),
        }
    }

    pub fn open(file: File) -> Self {
        let file = file;
        let mut file_rows = Vec::new();
        let mut count = Count {
            words: 0,
            chars: 0,
        };
        
        for line in file.as_string.lines() {
            let counts = words_count::count(&file.as_string);
            count.words = counts.words;
            count.chars = counts.characters;
            file_rows.push(FileRow::from(line));
        };

        let start_count = count.clone();

        Self { 
            file,
            file_rows,
            display_rows: Vec::new(),
            append_buffer: AppendBuffer::default(),
            last_edit: Instant::now(),
            count,
            start_count,
        }
    }

    pub fn save(&mut self, words: u8) -> Result<(), Error> {
        let mut tmp_path = self.file.path.clone();
        tmp_path.set_extension("tmp");
        if let Some(path_string) = tmp_path.to_str() {
            trace!("[file.rs]: using config path: {path_string}");
        }
        let mut save_file = std::fs::File::create(&tmp_path)?;
        let mut contents = String::new();

        for (index, row) in self.file_rows.iter().enumerate() {
            contents.push_str(&row.content);
            if index != self.file_rows.len().saturating_sub(1) {
                contents.push('\n');
            }
        };
        if words > 1 {
            if let Some((split_at_index, ..)) = self.append_buffer.buffer
            // BAD?: should i use split_inclusive() here instead?
            .unicode_word_indices()
            .nth(words.saturating_sub(1) as usize) {
                let split_string = self.append_buffer.buffer
                .split_at(split_at_index);
                let first_five_words = split_string.0;
                let remainder = split_string.1; 

                contents.push_str(first_five_words);

                self.append_buffer.buffer = remainder.to_string();
            };
        } else if !self.append_buffer.buffer.is_empty() {
            contents.push_str(&self.append_buffer.buffer);
            self.append_buffer.buffer.clear();
        };
        // put a newline at the end of the file for
        // unix compliance :^)
        contents.push('\n');
        
        self.file.as_string = contents.clone();

        save_file.write_all(contents.as_bytes())?;

        self.last_edit = Instant::now();
        
        self.sync_file_rows();
        self.wrap_file();
        self.wrap_buffer();
        rename(&tmp_path, &self.file.path)?;
        Ok(())
    }

    pub fn sync_file_rows(&mut self) {
        let mut file_rows = Vec::new();
        let mut words = 0;
        let mut chars = 0;

        for line in self.file.as_string.lines() {
            let counts = words_count::count(&self.file.as_string);
            // .replace(&['\n', '\r'][..], " "));
            words = counts.words;
            chars = counts.characters;
            // word_count = content.unicode_words().count();
            // char_count = content
            // .split(&['\n', '\r'][..])
            // .collect::<String>()
            // .graphemes(true)
            // .count();
            file_rows.push(FileRow::from(line));
        }

        self.count.words = words;
        self.count.chars = chars;
        self.file_rows = file_rows;
    }

    // BAD: this currently reflows/renders the ENTIRE file
    // which is inefficient. currently works fine, but i'd
    // like to find a better solution
    pub fn wrap_file(&mut self) {
        trace!("wrapping file");
        self.display_rows.clear();
        
        let max_width = Terminal::get_term_size().0;
        let mut total_len = 0;
        
        for row in &self.file_rows {
            // wrap display line if necessary
            if row.len >= max_width {
                // split row into vector of substrings by word boundaries
                // let row_chunks = row.content[..].split_word_bounds().collect::<Vec<&str>>();
                let row_chunks = row.content[..].split_inclusive(' ').collect::<Vec<&str>>();
                
                // count graphemes in each element of the vector
                // then push to new vector including their length
                let mut counted_chunks = Vec::new();
                
                for chunk in row_chunks {
                    counted_chunks.push((chunk.graphemes(true).count(), chunk));
                }
                
                // concat chunks until size would become larger than max width
                // then push the resulting string as a display row and set the
                // size and chunk to the overflowing chunk
                let mut chunk_len = 0;
                let mut chunked_row = String::new();

                for chunk in counted_chunks {
                    if (chunk_len + chunk.0) < max_width {
                        chunk_len = chunk_len.saturating_add(chunk.0);
                        chunked_row.push_str(chunk.1);
                    } else {
                        total_len = chunk_len.saturating_add(total_len);

                        // this is a finished display-length row
                        let wrapped_row = DisplayRow {
                            content: chunked_row.clone(),
                            len: chunk_len,
                            is_buffer: false,
                        };

                        self.display_rows.push(wrapped_row);

                        chunk_len = chunk.0;
                        chunked_row = chunk.1.to_string();
                    }
                }
                total_len = chunk_len.saturating_add(total_len);
                // this is the remainder of a file row
                let wrapped_row = DisplayRow {
                    content: chunked_row.clone(),
                    len: chunk_len,
                    is_buffer: false,
                };
                
                self.display_rows.push(wrapped_row);
            } else {
                total_len = row.len.saturating_add(total_len);
                let display_row = DisplayRow { 
                    content: row.content.clone(),
                    len: row.len, 
                    is_buffer: false,
                };

                self.display_rows.push(display_row);
            }
        }
        if let Some(last_display_row) = self.display_rows.last() {
            self.append_buffer.last_drow = last_display_row.content.clone();
        };
        self.display_rows.pop();
    }

    pub fn wrap_buffer(&mut self) {
        trace!("wrapping buffer");
        let max_width = Terminal::get_term_size().0;
        let mut total_len = 0;

        // BAD: not ideal to run over the entire vector to clear
        // non buffer rows, but its better than reflowing the
        // entire file so i'll take it for now
        // check if row is part of the buffer or not       
        self.display_rows.retain(|row| !row.is_buffer);

        let to_wrap = format!("{}{}", self.append_buffer.last_drow, self.append_buffer.buffer);

        for line in to_wrap.lines() {
            // count the line's length in graphemes instead of chars or bytes
            let line_len = line.graphemes(true).count();
            
            // wrap if necessary
            if line_len >= max_width {
                // split line into vector of substrings by word boundaries
                // let line_chunks = line[..].split_word_bounds().collect::<Vec<&str>>();
                let line_chunks = line[..].split_inclusive(' ').collect::<Vec<&str>>();

                // count graphemes in each element of the vector
                // then push to new vector including their length
                let mut counted_chunks = Vec::new();

                for chunk in line_chunks {
                    counted_chunks.push((chunk.graphemes(true).count(), chunk));
                }

                // concat chunks until size would become larger than max width
                // then push resulting string as a display row and reset the
                // size and chunk to the overflow
                let mut chunk_len = 0;
                let mut chunked_row = String::new();

                for chunk in counted_chunks {
                    if (chunk_len + chunk.0) < max_width {
                        chunk_len = chunk_len.saturating_add(chunk.0);
                        chunked_row.push_str(chunk.1);
                    } else {
                        total_len = chunk_len.saturating_add(total_len);

                        // this is a finished wrapped display row
                        let wrapped_row = DisplayRow {
                            content: chunked_row.clone(),
                            len: chunk_len,
                            is_buffer: true,
                        };

                        self.display_rows.push(wrapped_row);

                        chunk_len = chunk.0;
                        chunked_row = chunk.1.to_string();
                    }
                }
                total_len = chunk_len.saturating_add(total_len);
                // this is the remainder of a line
                let wrapped_row = DisplayRow {
                    content: chunked_row.clone(),
                    len: chunk_len,
                    is_buffer: true,
                };
                
                self.display_rows.push(wrapped_row);
            } else {
                total_len = line_len.saturating_add(total_len);
                let display_row = DisplayRow { 
                    content: line.to_string(),
                    len: line_len, 
                    is_buffer: true,
                };

                self.display_rows.push(display_row);
            }
        }
        // if the end of the buffer is a newline,
        // add an extra row so lines() doesn't cut it off
        if self.append_buffer.buffer.ends_with('\n') {
            let display_row = DisplayRow { 
                content: String::new(),
                len: 0, 
                is_buffer: true,
            };

            self.display_rows.push(display_row);
        };

        // if the buffer and the last file row are both
        // empty, push an extra display row so files
        // with more than one newline at the end aren't
        // displayed wrong
        if let Some(last_frow) = self.file_rows.last() {
            if self.append_buffer.buffer.is_empty()
            && last_frow.content.is_empty() {
                trace!("[document.rs]: pushing extra display row");
                let display_row = DisplayRow { 
                    content: String::new(),
                    len: 0, 
                    is_buffer: true,
                };
    
                self.display_rows.push(display_row);
            };
        };
    }

    pub fn split_last_row(&self, row: &DisplayRow) -> (String, String) {
        let mut content = row.content.clone();
        let row_len = row.content.len();
        let buffer_len = self.append_buffer.buffer.len();
        let truncate_len = row_len.saturating_sub(buffer_len);
        let buffer = self.append_buffer.buffer.clone();
        
        content.truncate(truncate_len);

        (content, buffer)
    }

    pub fn insert(&mut self, char: char) {
        self.append_buffer.insert(char);
        self.wrap_buffer();
    }

    pub fn delete(&mut self) {
        self.append_buffer.delete();
        self.wrap_buffer();
    }

    pub fn get_display_row(&self, index: usize) -> Option<&DisplayRow> {
        self.display_rows.get(index)
    }

    pub fn written_this_session(&self) -> Count {
        let words_written = self.count.words.saturating_sub(self.start_count.words);
        let chars_written = self.count.chars.saturating_sub(self.start_count.chars);
        
        Count {
            words: words_written,
            chars: chars_written,            
        }
    }
}

pub fn render(to_render: &str) -> String {
    let mut rendered = String::new();

    for grapheme in to_render[..]
    .graphemes(true) {
        if grapheme == "\t" {
            rendered.push_str("  ");
        } else {
            rendered.push_str(grapheme);
        }
    }

    rendered
}
