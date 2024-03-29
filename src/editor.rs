use crate::{die, Terminal, Document, render, Metadata, Config};
use {
    std::{
        io::Error,
        time::{Duration, Instant},
        env::consts::OS,
        cmp::PartialEq,
    },
    crossterm::event::{Event, read, poll, KeyEvent, KeyEventKind, KeyModifiers, KeyCode},
    log::{error, trace},
};

// -----------------

const STANDARD_MESSAGE: &str = "help: press ctrl+h for keybinds";

// cursor_pos is only really used if the cursor
// needs to be placed somewhere special (e.g. in the prompt)
pub struct Editor {
    terminal: Terminal,
    document: Document,
    cursor_pos: Position,
    view_pos: Position,
    message: Message,
    should_quit: bool,
    mode: Mode,
    quit_times: u8,
    config: Config,
}

#[derive(Default)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(PartialEq, Clone)]
enum Mode {
    View,
    Edit,
    Prompt,
}

#[derive(PartialEq, Clone, Copy)]
pub enum SaveType {
    Words,
    Time,
    Manual,
}

struct Message {
    text: String,
    time: Instant,
}

impl Message {
    fn from(message: String) -> Self {
        Self {
            text: message,
            time: Instant::now(),
        }
    }
}

// i think other things in brr will break before
// we get to the point where the program can edit
// a file with a number of lines with more than 255
// digits (the max of u8)
#[allow(clippy::cast_possible_truncation)]
impl Editor {
    pub fn default(file: Metadata, config: Config) -> Self {
        let initial_message = String::from(STANDARD_MESSAGE);
        let mode = if config.start_edit {
            Mode::Edit
        } else {
            Mode::View
        };
        let quit_times = config.quit_times;
        let mut document;

        document = Document::open(file);
        Document::wrap_file(&mut document);
        Document::wrap_buffer(&mut document);
        
        Self {
            terminal: Terminal::default(),
            document,
            cursor_pos: Position::default(),
            view_pos: Position::default(),
            message: Message::from(initial_message),
            should_quit: false,
            mode,
            quit_times,
            config,
        }
    }

    pub fn run(&mut self) {
        if let Err(error_msg) = Terminal::init() {
            error!("[terminal.rs -> editor.rs]: {error_msg} - could not init terminal.");
            die(error_msg);
        };
        if let Err(error_msg) = self.terminal.set_cursor_style(self.config.cursor_style) {
            error!("[terminal.rs -> editor.rs]: {error_msg} - could not set cursor style.");
        }
        // snap view to end of document.
        if self.mode == Mode::View {
            self.view_pos.y = self.document.file_drows.len().saturating_sub(1);
        }
        self.snap_view();

        loop {
            if let Err(error_msg) = self.refresh_screen() {
                error!("[editor.rs]: {error_msg} - could not refresh screen.");
                die(error_msg);
            };
            if self.should_quit {
                let total_prose = &self.document.count;
                let session_prose = self.document.written_this_session();
                let quit_msg = if self.config.count_on_quit {
                    format!(
                        "goodbye!\r\napprox. total prose in {}:\r\n  {} words\r\n  {} chars\r\nwritten this session:\r\n  {} words\r\n  {} chars\r\n", 
                        self.document.metadata.name,
                        total_prose.words,
                        total_prose.characters,
                        session_prose.words, 
                        session_prose.characters,
                    )
                } else {
                    "goodbye!\r\n".to_string()
                };
                self.document.append_newline();
                if let Err(error_msg) = Terminal::quit(quit_msg) {
                    error!("[terminal.rs -> editor.rs]: {error_msg} - could not quit terminal.");
                    die(error_msg);
                };
                break;
            };
            self.process_event();
        };
    }

    // BAD: errors could be handled better here.
    pub fn refresh_screen(&mut self) -> Result<(), Error> {
        self.terminal.cursor_hide()?;
        self.terminal.move_cursor(&Position::default())?;
        self.draw_rows()?;
        self.draw_status_bar()?;
        self.draw_message_bar()?;
        if self.mode == Mode::Edit {
            self.terminal.move_cursor(&Position {
                x: self.view_pos.x,
                y: self.terminal.height / 2,
            })?;
            self.terminal.cursor_show()?;
        };
        self.terminal.flush()?;
        Ok(())
    }

    pub fn process_event(&mut self) {
        let event = read();

        match event {
            Ok(Event::Key(key)) => if OS == "windows" {
                self.windows_keypress(key);
            } else {
                self.process_keypress(key);
            },
            Ok(Event::Resize(first_x, first_y)) => self.term_resize(first_x as usize, first_y as usize),
            Err(error_msg) => error!("[editor.rs::process_event()]: {error_msg} - could not read event."),
            _ => (),
        }
    }

    pub fn term_resize(&mut self, first_x: usize, first_y: usize) {
        let (mut final_x, mut final_y) = (first_x, first_y);
        
        if let Err(error_msg) = self.terminal.cursor_hide_now() {
            error!("[editor.rs]: {error_msg} - could not hide cursor for terminal resize.");
        }
        if let Err(error_msg) = self.terminal.clear_all() {
            error!("[editor.rs]: {error_msg} - could not clear terminal.");
        }
        loop {
            let poll_duration = Duration::from_millis(100);

            match poll(poll_duration) {
                Ok(true) => match read() {
                    Ok(Event::Resize(new_x, new_y )) => {
                        final_x = new_x as usize;
                        final_y = new_y as usize;
                    },
                    Err(error_msg) => error!("[editor.rs::term_resize()]: {error_msg} - could not read event."),
                    _ => (),
                },
                Ok(false) => break,
                Err(error_msg) => error!("[editor.rs::term_resize()]: {error_msg} - could not poll."),
            };

        }
        if final_x != self.terminal.width {
            self.document.wrap_file();
            self.document.wrap_buffer();
        }

        self.terminal.width = final_x;
        self.terminal.height = final_y.saturating_sub(2);
        self.snap_view();
    }

    fn process_keypress(&mut self, key: KeyEvent) {
        let term_height = self.terminal.height;

        if self.mode != Mode::Prompt {
            match (key.modifiers, key.code) {
                (KeyModifiers::CONTROL, KeyCode::Char('q')) => {
                    if self.quit_times > 0 && self.document.append_buffer.is_dirty() {
                        self.message = 
                        Message::from(format!(
                            "file has unsaved changes. press ctrl-q {} more times to quit anyway.",
                            self.quit_times,
                        ));
                        
                        self.quit_times -= 1;
                        return
                    };
                    self.should_quit = true;
                },
                (KeyModifiers::CONTROL, KeyCode::Char('s')) => self.save(0, SaveType::Manual),
                (KeyModifiers::CONTROL, KeyCode::Char('o')) => self.open(),
                (KeyModifiers::CONTROL, KeyCode::Char('h')) => {
                    self.message = 
                    Message::from("ctrl+e - mode | ctrl+s - save | ctrl+o - open | ctrl+q - quit".to_string());
                },
                // editing mode
                (KeyModifiers::CONTROL, KeyCode::Char('e')) if self.mode == Mode::Edit => {
                    self.mode = Mode::View;
                    self.message = 
                    Message::from("arrow keys and pgup/down to navigate.".to_string());
                    self.snap_view();
                },
                (_, KeyCode::Char(pressed_char)) if self.mode == Mode::Edit => {
                    self.document.append_buffer.count_words();
                    if self.document.append_buffer.word_count == self.config.save_words as usize
                    && self.config.save_words > 1 {
                        self.save(self.config.save_words, SaveType::Words);
                    } else if self.document.last_edit.elapsed() > Duration::new(u64::from(self.config.save_time), 0)
                    && !self.document.append_buffer.buffer.is_empty() 
                    && self.config.save_time > 0 {
                        self.save(0, SaveType::Time);
                    }
                    self.document.insert(pressed_char);
                    self.snap_view();
                    self.document.last_edit = Instant::now();
                },
                (_, KeyCode::Backspace) if self.mode == Mode::Edit => {
                    // skip checking word count if backspacing
                    if self.document.last_edit.elapsed() > Duration::new(u64::from(self.config.save_time), 0)
                    && !self.document.append_buffer.buffer.is_empty() 
                    && self.config.save_time > 0 {
                        self.save(0, SaveType::Time);
                    }
                    if (self.view_pos.x > 0 
                    || self.view_pos.y > 0)
                    && !self.document.append_buffer.buffer.is_empty() {
                        self.document.delete();
                        self.snap_view();
                    };
                    self.document.last_edit = Instant::now();
                },
                (_, KeyCode::Enter) if self.mode == Mode::Edit => {
                    self.document.append_buffer.count_words();
                    if self.document.append_buffer.word_count == self.config.save_words as usize 
                    && self.config.save_words > 1 {
                        self.save(self.config.save_words, SaveType::Words);
                    } else if self.document.last_edit.elapsed() > Duration::new(u64::from(self.config.save_time), 0)
                    && !self.document.append_buffer.buffer.is_empty() 
                    && self.config.save_time > 0 {
                        self.save(0, SaveType::Time);
                    }
                    self.document.insert('\n');
                    self.snap_view();
                    self.document.last_edit = Instant::now();
                },
                // viewing mode
                (KeyModifiers::CONTROL, KeyCode::Char('e')) if self.mode == Mode::View => {
                    self.mode = Mode::Edit;
                    self.message = Message::from(STANDARD_MESSAGE.to_string());
                    self.snap_view();
                },
                (_, KeyCode::Up) if self.mode == Mode::View => self.viewing_scroll(&Direction::Up, 1),
                (_, KeyCode::Down) if self.mode == Mode::View => self.viewing_scroll(&Direction::Down, 1),
                (_, KeyCode::PageUp) if self.mode == Mode::View => self.viewing_scroll(&Direction::Up, term_height),
                (_, KeyCode::PageDown) if self.mode == Mode::View => self.viewing_scroll(&Direction::Down, term_height),
                _ => (),
            };
        };

        // if user presses anything but ctrl+q again, abort quitting by resetting
        // self.quit_times and message
        if self.quit_times < self.config.quit_times {
            self.quit_times = self.config.quit_times;
            self.message = Message::from(String::new());
        };
    }

    // BAD: i had to duplicate process keypress here because of a known issue
    // for windows terminals with crossterm. if i implemented this directly above,
    // it breaks the quit_times logic. i hate this solution but its good enough for
    // now.
    // too bad!
    // check these for updates:
    // https://github.com/crossterm-rs/crossterm/issues/797
    // https://github.com/crossterm-rs/crossterm/issues/752
    // https://github.com/crossterm-rs/crossterm/pull/778
    fn windows_keypress(&mut self, key: KeyEvent) {
        let term_height = self.terminal.height;

        if self.mode != Mode::Prompt {
            match (key.kind, key.modifiers, key.code) {
                (KeyEventKind::Press, KeyModifiers::CONTROL, KeyCode::Char('q')) => {
                    if self.quit_times > 0 && self.document.append_buffer.is_dirty() {
                        self.message = 
                        Message::from(format!(
                            "file has unsaved changes. press ctrl-q {} more times to quit anyway.",
                            self.quit_times,
                        ));
                        
                        self.quit_times -= 1;
    
                        return
                    };
                    self.should_quit = true;
                },
                (KeyEventKind::Press, KeyModifiers::CONTROL, KeyCode::Char('s')) => self.save(0, SaveType::Manual),
                (KeyEventKind::Press, KeyModifiers::CONTROL, KeyCode::Char('o')) => self.open(),
                (KeyEventKind::Press, KeyModifiers::CONTROL, KeyCode::Char('h')) => {
                    self.message = 
                    Message::from("ctrl+e - mode | ctrl+s - save | ctrl+o - open | ctrl+q - quit".to_string());
                },
                // editing mode
                (KeyEventKind::Press, KeyModifiers::CONTROL, KeyCode::Char('e')) if self.mode == Mode::Edit => {
                    self.mode = Mode::View;
                    self.message = 
                    Message::from("arrow keys and pgup/down to navigate.".to_string());
                    self.snap_view();
                },
                (KeyEventKind::Press, _, KeyCode::Char(pressed_char)) if self.mode == Mode::Edit => {
                    self.document.append_buffer.count_words();
                    if self.document.append_buffer.word_count == self.config.save_words as usize 
                    && self.config.save_words > 1 {
                        self.save(self.config.save_words, SaveType::Words);
                    } else if self.document.last_edit.elapsed() > Duration::new(u64::from(self.config.save_time), 0)
                    && !self.document.append_buffer.buffer.is_empty() 
                    && self.config.save_time > 0 {
                        self.save(0, SaveType::Time);
                    }
                    self.document.insert(pressed_char);
                    self.snap_view();
                    self.document.last_edit = Instant::now();
                },
                (KeyEventKind::Press, _, KeyCode::Backspace) if self.mode == Mode::Edit => {
                    // skip checking word count if backspacing
                    if self.document.last_edit.elapsed() > Duration::new(u64::from(self.config.save_time), 0)
                    && !self.document.append_buffer.buffer.is_empty() 
                    && self.config.save_time > 0 {
                        self.save(0, SaveType::Time);
                        self.message = Message::from("sorry, five seconds passed! file saved.".to_string());
                    }
                    if (self.view_pos.x > 0 
                    || self.view_pos.y > 0)
                    && !self.document.append_buffer.buffer.is_empty() {
                        self.document.delete();
                        self.snap_view();
                    };
                    self.document.last_edit = Instant::now();
                },
                (KeyEventKind::Press, _, KeyCode::Enter) if self.mode == Mode::Edit => {
                    self.document.append_buffer.count_words();
                    if self.document.append_buffer.word_count == self.config.save_words as usize 
                    && self.config.save_words > 1 {
                        self.save(self.config.save_words, SaveType::Words);
                    } else if self.document.last_edit.elapsed() > Duration::new(u64::from(self.config.save_time), 0)
                    && !self.document.append_buffer.buffer.is_empty() 
                    && self.config.save_time > 0{
                        self.save(0, SaveType::Time);
                    }
                    self.document.insert('\n');
                    self.snap_view();
                    self.document.last_edit = Instant::now();
                },
                // viewing mode
                (KeyEventKind::Press, KeyModifiers::CONTROL, KeyCode::Char('e')) if self.mode == Mode::View => {
                    self.mode = Mode::Edit;
                    self.message = Message::from(STANDARD_MESSAGE.to_string());
                    self.snap_view();
                },
                (KeyEventKind::Press, _, KeyCode::Up) if self.mode == Mode::View => self.viewing_scroll(&Direction::Up, 1),
                (KeyEventKind::Press, _, KeyCode::Down) if self.mode == Mode::View => self.viewing_scroll(&Direction::Down, 1),
                (KeyEventKind::Press, _, KeyCode::PageUp) if self.mode == Mode::View => self.viewing_scroll(&Direction::Up, term_height),
                (KeyEventKind::Press, _, KeyCode::PageDown) if self.mode == Mode::View => self.viewing_scroll(&Direction::Down, term_height),
                _ => (),
            };
        };
    }

    // snap view so end of file is at the middle of the screen
    // if in editing mode, also snap cursor to end of line
    fn snap_view(&mut self) {
        let term_height = self.terminal.height;
        let max_height = self.document.display_len();
        let last_display_index = max_height.saturating_sub(1);
        let gutter_len = self.document.line_no_digits.saturating_add(1);

        if self.mode == Mode::Edit {
            self.view_pos.y = last_display_index;
            // we should always be in the buffer in edit mode,
            // so it should be safe to just get a buffer drow
            // at the last index here
            if let Some(last_buf_drow) = self.document.buf_drows.last() {
                if self.config.line_numbers {
                    self.view_pos.x = last_buf_drow.len.saturating_add(gutter_len);
                } else {
                    self.view_pos.x = last_buf_drow.len;
                };
            } else if let Some(last_file_drow) = self.document.file_drows.last() {
                if self.config.line_numbers {
                    self.view_pos.x = last_file_drow.len.saturating_add(gutter_len);
                } else {
                    self.view_pos.x = last_file_drow.len;
                };
            } else {
                trace!("no last row");
                if self.config.line_numbers {
                    self.view_pos.x = 0_usize.saturating_add(gutter_len);
                } else {
                    self.view_pos.x = 0;
                };
            };
        } else if self.mode == Mode::View {
            if self.view_pos.y > max_height.saturating_sub((term_height / 2).saturating_add(1)) {
                self.view_pos.y = max_height.saturating_sub((term_height / 2).saturating_add(1));
            };
        } else if self.view_pos.y > max_height.saturating_sub((term_height / 2).saturating_add(1)) {
            self.view_pos.y = max_height.saturating_sub((term_height / 2).saturating_add(1));
        };
    }

    fn viewing_scroll(&mut self, direction: &Direction, amount: usize) {
        let max_height = self.document.display_len();
        let term_height = self.terminal.height;
        let position = &mut self.view_pos;

        match direction {
            Direction::Up => position.y = position.y.saturating_sub(amount),
            Direction::Down => position.y = position.y.saturating_add(amount),
            _ => (),
        };

        // stop view from scrolling too far past the end of the file
        if position.y > max_height.saturating_sub((term_height / 2).saturating_add(1)) {
            position.y = max_height.saturating_sub((term_height / 2).saturating_add(1));
        };
    }

    fn draw_rows(&mut self) -> Result<(), Error> {
        let term_height = self.terminal.height;
        let editing_offset = self.terminal.height / 2;
        let mut last_line_no = 0;

        for term_row in 0..term_height {
            // draw rows in view mode
            if self.mode == Mode::View {
                let index_to_display = self.view_pos.y.saturating_add(term_row);

                // since print_row() returns the line number of the row it's working on
                // i can set last_line_no and print the row at the same time
                last_line_no = self.print_row(index_to_display, last_line_no)?;

                // match self.document.get_display_row(index_to_display) {
                //     (Some(file_drow), None) => {
                //         let file_content = render(&file_drow.content);
                //         let to_print = format!(" {file_content}");
                //         if last_line_no == file_drow.line_no {
                //             gutter = " ".repeat(line_no_digits);
                //             self.terminal.queue_print_reversed(&gutter)?;
                //         } else {
                //             gutter = format!("{:>line_no_digits$}", file_drow.line_no.to_string());
                //             self.terminal.queue_print_reversed(&gutter)?;
                //         }
                //         last_line_no = file_drow.line_no;
                //         self.terminal.queue_print(&to_print)?;
                //     },
                //     (None, Some(buf_drow)) => {
                //         let buf_content = render(&buf_drow.content);
                //         let to_print = format!(" {buf_content}");
                //         if last_line_no == buf_drow.line_no {
                //             gutter = " ".repeat(line_no_digits);
                //             self.terminal.queue_print_reversed(&gutter)?;
                //         } else {
                //             gutter = format!("{:>line_no_digits$}", buf_drow.line_no.to_string());
                //             self.terminal.queue_print_reversed(&gutter)?;
                //         }
                //         last_line_no = buf_drow.line_no;
                //         self.terminal.queue_print(&to_print)?;
                //     },
                //     (Some(file_drow), Some(buf_drow)) => {
                //         let file_content = render(&file_drow.content);
                //         let buf_content = render(&buf_drow.content);
                //         let to_print = format!(" {file_content}{buf_content}");
                //         if last_line_no == file_drow.line_no {
                //             let gutter = " ".repeat(self.document.line_no_digits);
                //             self.terminal.queue_print_reversed(&gutter)?;
                //         } else {
                //             self.terminal.queue_print_reversed(&file_drow.line_no.to_string())?;
                //         }
                //         last_line_no = file_drow.line_no;
                //         self.terminal.queue_print(&render(&to_print))?;
                //     },
                //     (None, None) => {
                //         let gutter = " ".repeat(self.document.line_no_digits);
                //         self.terminal.queue_print_reversed(&gutter)?;
                //         self.terminal.queue_print(" ~")?;
                //     },
                // };
            // draw rows in edit/prompt mode
            // by checking if ((view position's y + current term row) - editing offset) 
            // does not overflow (read: would not become less than zero), we can make sure
            // to keep the view in the middle of the screen and print '~' before it
            } else if let Some(index_to_display) = self.view_pos.y.saturating_add(term_row).checked_sub(editing_offset) {
                // since print_row() returns the line number of the row it's working on
                // i can set last_line_no and print the row at the same time
                last_line_no = self.print_row(index_to_display, last_line_no)?;

                // match self.document.get_display_row(index_to_display) {
                //     (Some(file_drow), None) => {
                //         let file_content = render(&file_drow.content);
                //         let to_print = format!(" {file_content}");
                //         if last_line_no == file_drow.line_no {
                //             gutter = " ".repeat(line_no_digits);
                //             self.terminal.queue_print_reversed(&gutter)?;
                //         } else {
                //             gutter = format!("{:>line_no_digits$}", file_drow.line_no.to_string());
                //             self.terminal.queue_print_reversed(&gutter)?;
                //         }                      
                //         self.terminal.queue_print(&to_print)?;
                //     },
                //     (None, Some(buf_drow)) => {
                //         if buf_drow.content.is_empty() {
                //             self.terminal.queue_print_reversed(" ")?;
                //         } else {
                //             self.terminal.reverse_colors()?;
                //             self.terminal.queue_print(&render(&buf_drow.content))?;
                //             self.terminal.no_reverse_colors()?;
                //         }
                //     },
                //     (Some(file_drow), Some(buf_drow)) => {
                //         self.terminal.queue_print(&render(&file_drow.content))?;
                //         self.terminal.reverse_colors()?;
                //         self.terminal.queue_print(&render(&buf_drow.content))?;
                //         self.terminal.no_reverse_colors()?;
                //     },
                //     (None, None) => {
                //         let gutter = " ".repeat(self.document.line_no_digits);
                //         self.terminal.queue_print_reversed(&gutter)?;
                //         self.terminal.queue_print(" ~")?;
                //     },
                // };
            } else {
                if self.config.line_numbers {
                    let gutter = " ".repeat(self.document.line_no_digits);
                    self.terminal.queue_print_reversed(&gutter)?;
                    self.terminal.queue_print(" ")?;
                }
                self.terminal.queue_print("~")?;
            }
            self.terminal.clear_line()?;
            self.terminal.new_line()?;
        }
        Ok(())
    }

    fn print_row(&mut self, row_index: usize, last_line_no: usize) -> Result<usize, Error> {
        let line_no_digits = self.document.line_no_digits;
        let gutter;

        match self.document.get_display_row(row_index) {
            (Some(file_drow), None) => {
                let file_content = render(&file_drow.content);

                if self.config.line_numbers {
                    if last_line_no == file_drow.line_no {
                        gutter = " ".repeat(line_no_digits);
                        self.terminal.queue_print_reversed(&gutter)?;
                    } else {
                        gutter = format!("{:>line_no_digits$}", file_drow.line_no.to_string());
                        self.terminal.queue_print_reversed(&gutter)?;
                    };
                    self.terminal.queue_print(" ")?;
                };
                
                self.terminal.queue_print(&file_content)?;
                Ok(file_drow.line_no)
            },
            (None, Some(buf_drow)) => {
                let buf_content = render(&buf_drow.content);

                if self.config.line_numbers {
                    if last_line_no == buf_drow.line_no {
                        gutter = " ".repeat(line_no_digits);
                        self.terminal.queue_print_reversed(&gutter)?;
                    } else {
                        gutter = format!("{:>line_no_digits$}", buf_drow.line_no.to_string());
                        self.terminal.queue_print_reversed(&gutter)?;
                    };
                    self.terminal.queue_print(" ")?;
                };
                if self.mode == Mode::View {
                    self.terminal.queue_print(&buf_content)?;
                } else if buf_content.is_empty() {
                    self.terminal.queue_print_reversed(" ")?;
                } else {
                    self.terminal.queue_print_reversed(&buf_content)?;
                }
                Ok(buf_drow.line_no)
            },
            (Some(file_drow), Some(buf_drow)) => {
                let file_content = render(&file_drow.content);
                let buf_content = render(&buf_drow.content);

                if self.config.line_numbers {
                    if last_line_no == file_drow.line_no {
                        let gutter = " ".repeat(self.document.line_no_digits);
                        self.terminal.queue_print_reversed(&gutter)?;
                    } else {
                        self.terminal.queue_print_reversed(&file_drow.line_no.to_string())?;
                    }
                    self.terminal.queue_print(" ")?;
                }
                self.terminal.queue_print(&file_content)?;
                if self.mode == Mode::View {
                    self.terminal.queue_print(&buf_content)?;
                } else {
                    self.terminal.queue_print_reversed(&buf_content)?;
                }
                Ok(file_drow.line_no)
            },
            (None, None) => {
                if self.config.line_numbers {
                    let gutter = " ".repeat(self.document.line_no_digits);
                    self.terminal.queue_print_reversed(&gutter)?;
                    self.terminal.queue_print(" ")?;
                }
                self.terminal.queue_print("~")?;
                Ok(0)
            },
        }
    }

    fn draw_status_bar(&mut self) -> Result<(), Error>{
        // BAD: dividing this by three leads to the formatting getting screwed up
        // when the width isn't evenly divisible by three
        // let words = self.document.count.words;
        // let chars = self.document.count.chars;
        let mut file_name = self.document.metadata.name.clone();
        let dirty_indicator = if self.document.append_buffer.is_dirty() {
            "(*)"
        } else {
            ""
        };
        let mode_indicator = match self.mode {
            Mode::Edit => "EDITING",
            Mode::Prompt => "",
            Mode::View => "VIEWING",
        };
        // let count_indicator = format!(
        //     "{words} words / {chars} chars"
        // );

        let file_indicator = format!("{file_name} {dirty_indicator}");

        // BAD?: give some indication if the file name has been truncated?
        file_name.truncate(20);

        let max_width = self.terminal.width.saturating_sub(mode_indicator.len());

        let status_bar = format!(
            "{file_indicator:<max_width$}{mode_indicator}"
        );

        self.terminal.reverse_colors()?;
        self.terminal.queue_print(&status_bar)?;
        self.terminal.no_reverse_colors()?;
        self.terminal.new_line()?;
        Ok(())
    }

    fn draw_message_bar(&mut self) -> Result<(), Error> {
        self.terminal.clear_line()?;

        let message = &self.message;

        if message.time.elapsed() < Duration::new(5, 0) {
            let mut text = message.text.clone();

            text.truncate(self.terminal.width);
            self.terminal.queue_print(&text)?;
        }
        Ok(())
    }

    pub fn save(&mut self, words: u8, save_type: SaveType) {
        match self.document.save(words, save_type) {
            Ok(()) if save_type == SaveType::Manual => self.message = Message::from("file saved successfully.".to_string()),
            Ok(()) => (),
            Err(error_msg) => {
                self.message = Message::from("error writing file. see log for details.".to_string());
                error!("[document.rs -> editor.rs]: {error_msg} - could not save file.");
            },
        }
    }

    pub fn open(&mut self) {
        let prev_mode = self.mode.clone();
        self.mode = Mode::Prompt;
        let input = self.prompt(
            "file name: ", 11, |_, _, _| {}
        ).unwrap_or(None);
        
        if let Some(file_name) = input {
            self.document.append_newline();

            let file_info = if self.config.open_search {
                Metadata::get_file_info(&file_name, true)
            } else {
                Metadata::get_file_info(&file_name, false)
            };
            let mut document;

            document = Document::open(file_info);
            Document::wrap_file(&mut document);
            Document::wrap_buffer(&mut document);

            self.document = document;
            
            if self.config.start_edit {
                self.mode = Mode::Edit;
            } else {
                self.mode = Mode::View;
                // make sure view snaps to end of document
                self.view_pos.y = self.document.display_len().saturating_sub(1);
            }

            self.snap_view();
        } else {
            self.message = Message::from("open aborted".to_string());
            self.mode = prev_mode;
        };
    }

    // BAD: this whole closures and callbacks thing is a bit beyond me
    // so for now i'm just going to hope nothing breaks here
    // too bad!
    // BAD: when writing in the prompt, a very long input will cause
    // the cursor to move out of the screen or some other such funkyness
    fn prompt<C>(&mut self, prompt: &str, start_x: usize, mut callback: C) -> Result<Option<String>, Error> 
    where
        C: FnMut(&mut Self, KeyEvent, &String),
    {
        let mut user_input = String::new();
        // add one because i artificially shorten the terminal by two 
        // rows when getting the size in terminal.rs
        let message_bar_y = self.terminal.height.saturating_add(1); 
        self.cursor_pos = Position {
            x: start_x,
            y: message_bar_y,
        };

        loop {
            self.message = Message::from(format!("{prompt}{user_input}"));
            self.refresh_prompt()?;
            let event = read()?;

            // BAD: windows boilerplate
            if OS == "windows" {
                if let Event::Key(key) = event {
                    match (key.kind, key.code) {
                        (KeyEventKind::Press, KeyCode::Backspace) => {
                            user_input.truncate(user_input.len().saturating_sub(1));
                            self.cursor_pos = Position {
                                y: message_bar_y,
                                x: Editor::prompt_cursor_x(start_x, self.cursor_pos.x, &Direction::Left),
                            };
                        },
                        (KeyEventKind::Press, KeyCode::Enter) => break,
                        (KeyEventKind::Press, KeyCode::Char(character)) => {
                            if !character.is_control() {
                                user_input.push(character);
                                self.cursor_pos = Position {
                                    y: message_bar_y,
                                    x: Editor::prompt_cursor_x(start_x, self.cursor_pos.x, &Direction::Right),
                                };
                            }
                        },
                        (KeyEventKind::Press, KeyCode::Esc) => {
                            user_input.truncate(0);
                            break;
                        },
                        _ => (),
                    };
                    callback(self, key, &user_input);
                }
            } else if let Event::Key(key) = event {
                match key.code {
                    KeyCode::Backspace => {
                        user_input.truncate(user_input.len().saturating_sub(1));
                        self.cursor_pos = Position {
                            y: message_bar_y,
                            x: Editor::prompt_cursor_x(start_x, self.cursor_pos.x, &Direction::Left),
                        };
                    },
                    KeyCode::Enter => break,
                    KeyCode::Char(character) => {
                        if !character.is_control() {
                            user_input.push(character);
                            self.cursor_pos = Position {
                                y: message_bar_y,
                                x: Editor::prompt_cursor_x(start_x, self.cursor_pos.x, &Direction::Right),
                            };
                        }
                    },
                    KeyCode::Esc => {
                        user_input.truncate(0);
                        break;
                    },
                    _ => (),
                };
                callback(self, key, &user_input);
            }
        }
        self.message = Message::from(String::new());
        
        if user_input.is_empty() {
            return Ok(None);
        }
        Ok(Some(user_input))
    }

    fn prompt_cursor_x(start_x: usize, at_x: usize, direction: &Direction) -> usize {
        match direction {
            Direction::Left => if at_x.saturating_sub(1) < start_x {
                return start_x;
            } else {
                return at_x.saturating_sub(1);
            },
            Direction::Right => return at_x.saturating_add(1),
            _ => (),
        };
        0
    }

    fn refresh_prompt(&mut self) -> Result<(), Error> {
        self.terminal.cursor_hide()?;
        self.terminal.move_cursor(&Position { 
            x: 0, 
            y: self.terminal.height, 
        })?;
        self.draw_status_bar()?;
        self.draw_message_bar()?;
        self.terminal.move_cursor(&self.cursor_pos)?;
        self.terminal.cursor_show()?;
        self.terminal.flush()?;
        Ok(())
    }
}
