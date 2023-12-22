use crate::{Terminal, Metadata, DisplayRow, AppendBuffer, Position, die};
use {
    unicode_segmentation::UnicodeSegmentation,
    words_count::WordsCount,
    log::{error, warn, info, trace},
    std::{
        cmp::Ordering,
        time::Instant,
        io::{Read, Seek, Error, Write, BufReader, BufWriter},
        fs::{OpenOptions, rename, File},
    }
};

// -----------------

pub struct Document {
    pub metadata: Metadata,
    pub content: String,
    pub file_drows: Vec<DisplayRow>,
    pub append_buffer: AppendBuffer,
    pub buf_drows: Vec<DisplayRow>,
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
            // on windows, some files will end with \r\n, so pop
            // that bad boy off too.
            if content.ends_with('\r') {
                content.pop();
            }
        }

        let start_count = count.clone();

        Self { 
            metadata,
            content: content.to_string(),
            file_drows: Vec::new(),
            append_buffer: AppendBuffer::default(),
            buf_drows: Vec::new(),
            last_edit: Instant::now(),
            count,
            start_count,
        }
    }

    pub fn save(&mut self, words: u8) -> Result<(), Error> {
        let mut tmp_path = self.metadata.path.clone();
        tmp_path.set_extension("tmp");
        info!(
            "[document.rs]: saving temp file at path {}",
            &tmp_path.display()
        );
        let mut save_file = BufWriter::new(File::create(&tmp_path)?);

        // words here is set in the config file and
        // refers to how many words brr should save
        // as they're written
        if words > 1 {
            if let Some((split_at_index, ..)) = self.append_buffer.buffer
            .unicode_word_indices()
            .nth(words.saturating_sub(1) as usize) {
                let (to_save, remainder) = self.append_buffer.buffer
                .split_at(split_at_index);

                self.content.push_str(to_save);
                self.append_buffer.buffer = remainder.to_string();
            };
        } else if !self.append_buffer.buffer.is_empty() {
            self.content.push_str(&self.append_buffer.buffer);
            self.append_buffer.buffer.clear();
        };

        self.count = words_count::count(&self.content);
        
        save_file.write_all(self.content.as_bytes())?;

        self.wrap_file();
        self.wrap_buffer();
        rename(&tmp_path, &self.metadata.path)?;

        self.last_edit = Instant::now();

        Ok(())
    }

    // BAD: this currently reflows the ENTIRE file whenever 
    // brr saves. it's mitigated by this not happening on 
    // every keypress, but it still happens often enough that 
    // i would like to find a better solution
    pub fn wrap_file(&mut self) {
        self.file_drows.clear();
        self.file_drows = to_display_rows(0,&self.content);

        let last_drow_index = self.file_drows.len().saturating_sub(1);

        // if there are extra newlines at the end of the file,
        // lines() being called in to_display_rows() would remove
        // one of them. so we add an extra here and inform the 
        // append buffer accordingly
        if self.content.ends_with('\n') {
            trace!("newline at end of file");
            self.append_buffer.join_pos = Position {
                x: 0,
                y: last_drow_index.saturating_add(1),
            };
            
            self.file_drows.push(DisplayRow::from((String::new(), 0)));
        // otherwise inform the append buffer exactly where it
        // starts within the display rows
        } else if let Some(last_drow) = self.file_drows.last() {    
            self.append_buffer.join_pos = Position {
                x: last_drow.len,
                y: last_drow_index,
            };
            // self.append_buffer.join_content = last_drow.content.clone();
            
            // self.file_drows.pop();
        };        
    }

    pub fn wrap_buffer(&mut self) {
        self.buf_drows.clear();
        self.buf_drows = to_display_rows(
            self.append_buffer.join_pos.x, 
            &self.append_buffer.buffer
        );

        // if there is a newline at the end of the buffer,
        // add an extra display row so that it doesn't get
        // cut off from running lines() in to_display_rows()
        if self.append_buffer.buffer.ends_with('\n') {
            trace!("newline at end of buffer");
            self.buf_drows.push(DisplayRow::from((String::new(), 0)));
        };
    }

    pub fn get_display_row(&self, index: usize) -> (Option<&DisplayRow>, Option<&DisplayRow>) {
        let join_index = self.append_buffer.join_pos.y;
        let file_drow_count = self.file_drows.len();
        // this translates the passed index to one within
        // the buffer by subtracting the total amount of
        // file display rows from it and adding one to
        // account for the joining row
        let buf_index_from_index = index.saturating_sub(file_drow_count);
        let file_drow = self.file_drows.get(index);
        
        match index.cmp(&join_index) {
            Ordering::Less => (file_drow, None),
            Ordering::Greater => (
                None, 
                self.buf_drows.get(buf_index_from_index.saturating_add(1))
            ),
            Ordering::Equal => (
                file_drow,
                self.buf_drows.get(buf_index_from_index)
            ),
        }
    }

    pub fn insert(&mut self, char: char) {
        self.append_buffer.insert(char);
        self.wrap_buffer();
    }

    pub fn delete(&mut self) {
        self.append_buffer.delete();
        self.wrap_buffer();
    }

    pub fn display_len(&self) -> usize {
        // subtract 1 because the joining row technically exists twice
        if self.buf_drows.len() <= 1 {
            return self.file_drows.len()
        }
        self.file_drows.len().saturating_add(self.buf_drows.len()).saturating_sub(1)
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

// wraps a string to display rows
pub fn to_display_rows(start_len: usize, to_wrap: &str) -> Vec<DisplayRow> {
    // get terminal width
    let max_width = Terminal::get_term_size().0;
    // create vector to return
    let mut display_rows = Vec::new();

    // split string into lines by \n or \r\n
    for line in to_wrap.lines() {
        // count length of the line in graphemes so we know
        // how long it will actually display as
        let mut line_display_len = line.graphemes(true).count();

        // if we're on the first run of the loop (aka the
        // display_rows vector is still empty), add the
        // start length so we can display the first line
        // of the buffer correctly
        if display_rows.is_empty() {
            line_display_len = line.graphemes(true).count().saturating_add(start_len);
        }

        // if the line's display length is too wide,
        // start the wrapping process
        if line_display_len >= max_width {
            // start by splitting the line into chunks by spaces
            // BAD: should ideally split on something better 
            // for wide characters/other alphabets later. consider:
            // let line_chunks = line[..].split_word_bounds().collect::<Vec<&str>>();
            let line_chunks = line
            .split_inclusive(' ')
            .collect::<Vec<&str>>();
            // count display length of chunks and collect them
            let mut counted_chunks = Vec::new();
            
            for chunk in line_chunks {
                counted_chunks.push((chunk.graphemes(true).count(), chunk));
            };

            let mut row_display_len = 0;
            // if we're on the first run of the loop (aka the
            // display_rows vector is still empty), add the
            // start length so we can display the first line
            // of the buffer correctly
            if display_rows.is_empty() {
                row_display_len = start_len;
            }
            let mut row = String::new();

            // iterate over counted chunks 
            for (chunk_len, chunk) in counted_chunks {
                // if the total row length plus the length of the chunk
                // if less than the terminal width, add the chunk to the
                // row
                if (row_display_len + chunk_len) < max_width {
                    row_display_len = row_display_len.saturating_add(chunk_len);
                    row.push_str(chunk);
                // otherwise, the combined chunks are a finished row, so push them
                } else {
                    display_rows.push(DisplayRow::from((row, row_display_len)));

                    // reset the row length and row content to be the 
                    // remainder (aka the chunk that would have pushed the
                    // row over the max length)
                    row_display_len = chunk_len;
                    row = chunk.to_string();
                }
            };
            // make sure to push the remainder after the for loop
            // has completed
            display_rows.push(DisplayRow::from((row, row_display_len)));
        } else {
            // if the line isn't too long, just push it directly.
            display_rows.push(DisplayRow::from((line.to_string(), line_display_len)));
        };
    };
    display_rows
}
