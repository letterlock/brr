use crate::Terminal;
use crate::Metadata;
use crate::DisplayRow;
use crate::AppendBuffer;
use crate::die::die;

use log::{error, trace, warn};
use unicode_segmentation::UnicodeSegmentation;
use words_count::WordsCount;
use std::fs::OpenOptions;
use std::io::Read;
use std::io::Seek;
use std::{
    time::Instant,
    io::{
        Error,
        Write,
        BufReader,
    },
    fs::{
        rename,
        File,
    },
};
pub struct Document {
    pub metadata: Metadata,
    pub content: String,
    pub display_rows: Vec<DisplayRow>,
    pub append_buffer: AppendBuffer,
    pub last_edit: Instant,
    pub count: WordsCount,
    pub start_count: WordsCount,
}

impl Document {

    pub fn open(metadata: Metadata) -> Self {
        let mut content = String::new();
        let mut count = WordsCount {
            words: 0,
            characters: 0,
            ..Default::default()
        };

        match File::open(metadata.path.clone()) {
            Ok(to_open) => {
                let mut file = BufReader::new(to_open);

                if let Err(error_msg) = file.read_to_string(&mut content) { 
                    error!("[document.rs]: {error_msg} - could not read file to string.");
                    die(error_msg);
                };
                count = words_count::count(&content);
            },
            Err(error_msg) => {
                warn!(
                    "[document.rs]: {} - could not open file. creating a new one at path {}",
                    error_msg,
                    &metadata.path.display()
                );
            },
        };

        if content.ends_with('\n') {
            content.pop();
        }

        let start_count = count.clone();

        Self { 
            metadata,
            content: content.to_string(),
            display_rows: Vec::new(),
            append_buffer: AppendBuffer::default(),
            last_edit: Instant::now(),
            count,
            start_count,
        }
    }

    pub fn save(&mut self, words: u8) -> Result<(), Error> {
        let mut tmp_path = self.metadata.path.clone();
        tmp_path.set_extension("tmp");
        
        trace!(
            "[document.rs]: saving temp file at path {}",
            &tmp_path.display()
        );

        let mut save_file = File::create(&tmp_path)?;   

        if words > 1 {
            if let Some((split_at_index, ..)) = self.append_buffer.buffer
            // BAD?: should i use split_inclusive() here instead?
            .unicode_word_indices()
            .nth(words.saturating_sub(1) as usize) {
                let split_string = self.append_buffer.buffer
                .split_at(split_at_index);
                let first_five_words = split_string.0;
                let remainder = split_string.1; 

                self.content.push_str(first_five_words);
                self.append_buffer.buffer = remainder.to_string();
            };
        } else if !self.append_buffer.buffer.is_empty() {
            self.content.push_str(&self.append_buffer.buffer);
            self.append_buffer.buffer.clear();
        };

        self.count = words_count::count(&self.content);

        save_file.write_all(self.content.as_bytes())?;

        self.last_edit = Instant::now();
        
        // self.sync_file_rows();
        self.wrap_file();
        self.wrap_buffer();
        rename(&tmp_path, &self.metadata.path)?;
        Ok(())
    }

    // BAD: this currently reflows/renders the ENTIRE file
    // which is inefficient. currently works fine, but i'd
    // like to find a better solution
    pub fn wrap_file(&mut self) {
        self.display_rows.clear();
        
        let max_width = Terminal::get_term_size().0;
        let mut total_len = 0;
        
        for line in self.content.lines() {
            let line_len = line.graphemes(true).count();
            // wrap display line if necessary
            if line_len >= max_width {
                // split row into vector of substrings by word boundaries
                // let row_chunks = row.content.split_word_bounds().collect::<Vec<&str>>();
                
                let row_chunks = line.split_inclusive(' ').collect::<Vec<&str>>();
                
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
                total_len = line.len().saturating_add(total_len);
                let display_row = DisplayRow { 
                    content: line.to_string(),
                    len: line_len, 
                    is_buffer: false,
                };

                self.display_rows.push(display_row);
            }
        }

        // if the file ends with a newline, add an extra
        // display row so things display correctly
        if let Some(last_display_row) = &mut self.display_rows.last() {
            if self.content.ends_with('\n') {
                let display_row = DisplayRow { 
                    content: String::new(),
                    len: 0, 
                    is_buffer: true,
                };
    
                self.display_rows.push(display_row);
            } else {
                self.append_buffer.last_drow = last_display_row.content.clone();
                self.display_rows.pop();
            };
        };        
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
                let line_chunks = line.split_inclusive(' ').collect::<Vec<&str>>();

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
        if self.append_buffer.last_drow.is_empty()
        && self.append_buffer.buffer.is_empty() 
        || self.append_buffer.buffer.ends_with('\n'){
            let display_row = DisplayRow { 
                content: String::new(),
                len: 0, 
                is_buffer: true,
            };

            self.display_rows.push(display_row);
        };
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

    pub fn written_this_session(&self) -> WordsCount {
        let words_written = self.count.words.saturating_sub(self.start_count.words);
        let chars_written = self.count.characters.saturating_sub(self.start_count.characters);
        
        WordsCount {
            words: words_written,
            characters: chars_written,
            ..Default::default()
        }
    }

    // put a newline at the end of the file for
    // unix compliance :^)
    pub fn append_newline(&mut self) {
        let mut buffer = [0; 1];
        
        if let Ok(mut file) = OpenOptions::new()
        .read(true)
        .write(true)
        .append(true)
        .open(&self.metadata.path) {
            if file.seek(std::io::SeekFrom::End(-1)).is_err() {
                warn!("[document.rs]: file is empty.");
            }
            if let Err(error_msg) = file.read_exact(&mut buffer[..]) {
                error!("[document.rs]: {error_msg} - could not read end of file.");
            };
            if buffer != [b'\n'] {
                if let Err(error_msg) = writeln!(file) {
                    error!("[document.rs]: {error_msg} - could not append newline to end of file.");
                };
            }
        } else {
            error!("[document.rs]: could not open file to append newline.");
        };
    }
}

pub fn render(to_render: &str) -> String {
    let mut rendered = String::new();

    for grapheme in to_render
    .graphemes(true) {
        if grapheme == "\t" {
            rendered.push_str("  ");
        } else {
            rendered.push_str(grapheme);
        }
    }

    rendered
}
