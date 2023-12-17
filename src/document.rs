// use crate::die;
use crate::Terminal;
use crate::File;
use crate::FileRow;
use crate::DisplayRow;
use crate::AppendBuffer;

use unicode_segmentation::UnicodeSegmentation;
// use words_count::WordsCount;
use std::{
    time::Instant,
    // path::Path,
    io::{
        Error,
        Write,
    },
};

// #[derive(Default)]
pub struct Document {
    pub file: File,
    pub file_rows: Vec<FileRow>,
    pub display_rows: Vec<DisplayRow>,
    pub append_buffer: AppendBuffer,
    pub last_edit: Instant,
    pub word_count: usize,
    pub char_count: usize,
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
            word_count: 0,
            char_count: 0,
        }
    }

    pub fn open(file: File) -> Self {
        let file = file;
        let mut file_rows = Vec::new();
        let mut word_count = 0;
        let mut char_count = 0;

        
        for line in file.as_string.lines() {
            let counts = words_count::count(&file.as_string);
            word_count = counts.words;
            char_count = counts.characters;
            file_rows.push(FileRow::from(line));
        };

        Self { 
            file,
            file_rows,
            display_rows: Vec::new(),
            append_buffer: AppendBuffer::default(),
            last_edit: Instant::now(),
            word_count,
            char_count,
        }
    }

    pub fn save(&mut self, words: bool) -> Result<(), Error> {
        // BAD: brr should have some way to make sure your
        // file doesn't get screwed because of an error
        // on the program's side
        let mut save_file = std::fs::File::create(&self.file.name)?;
        let mut contents = String::new();

        for (index, row) in self.file_rows.iter().enumerate() {
            contents.push_str(&row.content);
            if index != self.file_rows.len().saturating_sub(1) {
                contents.push('\n');
            }
        };
        if words {
            if let Some((split_at_index, ..)) = self.append_buffer.buffer
            .unicode_word_indices()
            .nth(5) {
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
        Ok(())
    }

    pub fn sync_file_rows(&mut self) {
        let mut file_rows = Vec::new();
        let mut word_count = 0;
        let mut char_count = 0;

        for line in self.file.as_string.lines() {
            let counts = words_count::count(&self.file.as_string);
            // .replace(&['\n', '\r'][..], " "));
            word_count = counts.words;
            char_count = counts.characters;
            // word_count = content.unicode_words().count();
            // char_count = content
            // .split(&['\n', '\r'][..])
            // .collect::<String>()
            // .graphemes(true)
            // .count();
            file_rows.push(FileRow::from(line));
        }

        self.word_count = word_count;
        self.char_count = char_count;
        self.file_rows = file_rows;
    }

    // BAD: this currently reflows/renders the ENTIRE file
    // which is inefficient. currently works fine, but i'd
    // like to find a better solution
    pub fn wrap_file(&mut self) {
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
        // self.char_count = total_len;
    }

    pub fn wrap_buffer(&mut self) {
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
        }
        // self.append_buffer.char_count = total_len;
    }

    pub fn render_buffer(&self, row_to_render: &DisplayRow) -> Option<(String, String)> {
      let content = &row_to_render.content.clone();

      if content.contains(&self.append_buffer.last_drow) {
          let split_index = self.append_buffer.last_drow.len();
          let (last_drow, buffer) = content.split_at(split_index);
          let (mut rendered_last_drow, mut rendered_buffer) = (String::new(), String::new());

          for grapheme in last_drow[..]
          .graphemes(true) {
              if grapheme == "\t" {
                  rendered_last_drow.push_str("  ");
              } else {
                  rendered_last_drow.push_str(grapheme);
              }
          };
          for grapheme in buffer[..]
          .graphemes(true) {
              if grapheme == "\t" {
                  rendered_buffer.push_str("  ");
              } else {
                  rendered_buffer.push_str(grapheme);
              }
          };

          return Some((rendered_last_drow, rendered_buffer));
      }
      None
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
}
