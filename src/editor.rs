use crate::die;
use crate::Terminal;
use crate::Document;
use crate::row::DisplayRow;

use std::{
    path::Path,
    io::Error,
    time::{
        Duration,
        Instant,
    },
    env::consts::OS,
};
use crossterm::event::KeyEventKind;
use crossterm::event::{
    Event,
    read,
    poll,
    KeyEvent,
    KeyModifiers,
    KeyCode,
};

//const VERSION: &str = env!("CARGO_PKG_VERSION");
const QUIT_TIMES: u8 = 3;

// editing:
//  true = editing mode
// false = viewing mode
pub struct Editor {
    terminal: Terminal,
    document: Document,
    position: Position,
    message: Message,
    should_quit: bool,
    editing: bool,
    quit_times: u8,
}

// describes the cursor's position within the 
// document, not it's position within the terminal
#[derive(Default)]
pub struct Position {
    pub x: usize,
    pub y: usize,
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

enum ScrollDirection {
    Up,
    Down,
}

impl Editor {
    pub fn default(file_name: &str) -> Self {
        let file = Path::new(file_name);
        let initial_message = String::from(
            "help: press ctrl+h at any time for keybinds"
        );
        let mut document;

        if file.exists() {
            document = Document::open(file_name);
            Document::wrap_file(&mut document);
            Document::wrap_buffer(&mut document);
        } else {
            document = Document::create(file_name);
        }
        
        Self {
            terminal: Terminal::default(),
            document,
            position: Position::default(),
            message: Message::from(initial_message),
            should_quit: false,
            editing: false,
            quit_times: QUIT_TIMES,
        }
    }

    pub fn run(&mut self) {
        if let Err(error_msg) = Terminal::init() {
            die(error_msg);
        };
        loop {
            if let Err(error_msg) = self.refresh_screen() {
                die(error_msg);
            };
            if self.should_quit {
                if let Err(error_msg) = Terminal::quit() {
                    die(error_msg);
                };
                break;
            };
            self.process_event();
        };
    }

    pub fn save(&mut self, words: bool) {
        if !words {
            if let Ok(()) = self.document.save(words) {
                self.message = 
                Message::from("file saved successfully".to_string());
            } else {
                self.message =
                Message::from("error writing file".to_string());
            }
        } else if self.document.save(words).is_err() {
            self.message =
            Message::from("error writing file".to_string());
        }
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
            Err(error_msg) => die(error_msg),
            _ => (),
        }
    }

    pub fn term_resize(&mut self, first_x: usize, first_y: usize) {
        let (mut final_x, mut final_y) = (first_x, first_y);
        
        if let Err(error_msg) = self.terminal.cursor_hide_now() {
            die(error_msg);
        }
        if let Err(error_msg) = self.terminal.clear_all() {
            die(error_msg);
        }
        loop {
            let poll_duration = Duration::from_millis(100);

            match poll(poll_duration) {
                Ok(true) => match read() {
                    Ok(Event::Resize(new_x, new_y )) => {
                        final_x = new_x as usize;
                        final_y = new_y as usize;
                    },
                    Err(error_msg) => die(error_msg),
                    _ => (),
                },
                Ok(false) => break,
                Err(error_msg) => die(error_msg),
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
            (KeyModifiers::CONTROL, KeyCode::Char('s')) => self.save(false),
            (KeyModifiers::CONTROL, KeyCode::Char('h')) => {
                self.message = 
                Message::from("ctrl+e to toggle mode, ctrl+s to save, ctrl+q to quit.".to_string());
            },
            // editing mode
            (KeyModifiers::CONTROL, KeyCode::Char('e')) if self.editing => {
                self.editing = false;
                self.message = 
                Message::from("arrow keys and pgup/down to navigate.".to_string());
                self.snap_view();
            },
            (_, KeyCode::Char(pressed_char)) if self.editing => {
                self.document.append_buffer.count_words();
                if self.document.append_buffer.word_count > 6 {
                    self.save(true);
                } else if self.document.last_edit.elapsed() > Duration::new(5, 0)
                && !self.document.append_buffer.buffer.is_empty() {
                    self.save(false);
                    // self.message = Message::from("buffer saved".to_string());
                }
                self.document.insert(pressed_char);
                self.snap_view();
                self.document.last_edit = Instant::now();
            },
            (_, KeyCode::Backspace) if self.editing => {
                // skip checking word count if backspacing
                if self.document.last_edit.elapsed() > Duration::new(5, 0)
                && !self.document.append_buffer.buffer.is_empty() {
                    self.save(false);
                    // self.message = Message::from("buffer saved".to_string());
                }
                if self.position.x > 0 
                || self.position.y > 0 {
                    self.document.delete();
                    self.snap_view();
                };
                self.document.last_edit = Instant::now();
            },
            (_, KeyCode::Enter) if self.editing => {
                self.document.append_buffer.count_words();
                if self.document.append_buffer.word_count > 6 {
                    self.save(true);
                } else if self.document.last_edit.elapsed() > Duration::new(5, 0)
                && !self.document.append_buffer.buffer.is_empty() {
                    self.save(false);
                    // self.message = Message::from("buffer saved".to_string());
                }
                self.document.insert('\n');
                self.snap_view();
                self.document.last_edit = Instant::now();
            },
            // viewing mode
            (KeyModifiers::CONTROL, KeyCode::Char('e')) if !self.editing => {
                self.editing = true;
                self.snap_view();
            },
            (_, KeyCode::Up) if !self.editing => self.viewing_scroll(&ScrollDirection::Up, 1),
            (_, KeyCode::Down) if !self.editing => self.viewing_scroll(&ScrollDirection::Down, 1),
            (_, KeyCode::PageUp) if !self.editing => self.viewing_scroll(&ScrollDirection::Up, term_height),
            (_, KeyCode::PageDown) if !self.editing => self.viewing_scroll(&ScrollDirection::Down, term_height),
            _ => (),
        };

        // if user presses anything but ctrl+q again, abort quitting by resetting
        // self.quit_times and message
        if self.quit_times < QUIT_TIMES {
            self.quit_times = QUIT_TIMES;
            self.message = Message::from(String::new());
        };
    }

    // snap view so end of file is at the middle of the screen
    // if in editing mode, also snap cursor to end of line
    fn snap_view(&mut self) {
        let term_height = self.terminal.height;
        let max_height = self.document.display_rows.len();
        let last_display_index = self.document.display_rows.len().saturating_sub(1);

        if self.editing {
            self.position.y = last_display_index;
            if let Some(last_drow) = self.document.get_display_row(last_display_index) {
                self.position.x = last_drow.len;
            } else {
                self.position.x = 0;
            }
        } else if self.position.y > max_height.saturating_sub((term_height / 2).saturating_add(1)) {
            self.position.y = max_height.saturating_sub((term_height / 2).saturating_add(1));
        };
    }

    fn viewing_scroll(&mut self, direction: &ScrollDirection, amount: usize) {
        let max_height = self.document.display_rows.len().saturating_sub(1);
        let term_height = self.terminal.height;
        let mut position = &mut self.position;

        match direction {
            ScrollDirection::Up => position.y = position.y.saturating_sub(amount),
            ScrollDirection::Down => position.y = position.y.saturating_add(amount),
        };

        // stop view from scrolling too far past the end of the file
        if position.y > max_height.saturating_sub((term_height / 2).saturating_add(1)) {
            position.y = max_height.saturating_sub((term_height / 2).saturating_add(1));
        };
    }

    pub fn draw_rows(&mut self) -> Result<(), Error> {
        let term_height = self.terminal.height;
        let editing_offset = self.terminal.height / 2;

        // BAD?: lot of nested ifs here, maybe i can clean this up.
        for term_row in 0..term_height {
            // draw rows in edit mode
            if self.editing {
                // if ((y offset + current term row) - editing offset) does not overflow (read:
                // would not become less than zero), check if that row exists in file and print it.
                // otherwise check if that row exists in append buffer and print it. otherwise print ~
                if let Some(index_to_display) = self.position.y.saturating_add(term_row).checked_sub(editing_offset) {
                    if let Some(row_to_render) = self.document.get_display_row(index_to_display) {
                        // if we're rendering a row from the append buffer, check if it's the 
                        // joining row from the rest of the file. if so, split it and render
                        // the parts separately so we can invert the colours on the buffer only
                        if row_to_render.is_buffer {
                            if let Some((rendered_row, rendered_buffer)) = self.document.render_buffer(row_to_render) {
                                self.terminal.queue_print(&rendered_row)?;
                                self.terminal.reverse_colors()?;
                                self.terminal.queue_print(&rendered_buffer)?;
                                self.terminal.no_reverse_colors()?;
                            } else {
                                self.terminal.reverse_colors()?;
                                if row_to_render.content.is_empty() {
                                    self.terminal.queue_print(" ")?;
                                } else {
                                    self.terminal.queue_print(&DisplayRow::render(row_to_render))?;
                                }
                                self.terminal.no_reverse_colors()?;
                            };
                        } else {
                            self.terminal.queue_print(&DisplayRow::render(row_to_render))?;
                        } 
                    } else {
                        self.terminal.queue_print("~")?;
                    }
                } else {
                    self.terminal.queue_print("~")?;
                }
            // draw rows in view mode
            } else {
                let index_to_display = self.position.y.saturating_add(term_row);

                if let Some(row_to_render) = self.document.get_display_row(index_to_display) {
                    self.terminal.queue_print(&DisplayRow::render(row_to_render))?;
                } else {
                    self.terminal.queue_print("~")?;
                }
            }
            self.terminal.clear_line()?;
            self.terminal.new_line()?;
        }
        Ok(())
    }

    fn draw_status_bar(&mut self) -> Result<(), Error>{
        let mut status;
        let term_width = self.terminal.width;
        let mut file_name = self.document.file_name.clone();
        let word_count = self.document.word_count;
        let char_count = self.document.char_count;
        let dirty_indicator = if self.document.append_buffer.is_dirty() {
            "(*)"
        } else {
            ""
        };
        let mode = if self.editing {
            "EDITING"
        } else {
            "VIEWING"
        };
        file_name.truncate(20);

        status = format!(
            " {} - {} lines {} {}", 
            file_name, 
            self.document.file_rows.len(),
            dirty_indicator,
            mode,
        );

        let line_indicator = format!(
            "{word_count} words / {char_count} chars "
        );
        let status_len = status.len() + line_indicator.len();
        let padding = " ".repeat(term_width - status_len);
        
        if term_width > status_len {
            status.push_str(&padding);
        }
        status = format!("{status}{line_indicator}");
        status.truncate(term_width);
        self.terminal.reverse_colors()?;
        self.terminal.queue_print(&status)?;
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

    pub fn refresh_screen(&mut self) -> Result<(), Error> {
        self.terminal.cursor_hide()?;
        self.terminal.move_cursor(&Position::default())?;
        self.draw_rows()?;
        self.draw_status_bar()?;
        self.draw_message_bar()?;
        if self.editing {
            self.terminal.move_cursor(&Position {
                x: self.position.x,
                y: self.terminal.height / 2,
            })?;
            self.terminal.cursor_show()?;
        };
        self.terminal.flush()?;
        Ok(())
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
            (KeyEventKind::Press, KeyModifiers::CONTROL, KeyCode::Char('s')) => self.save(false),
            (KeyEventKind::Press, KeyModifiers::CONTROL, KeyCode::Char('h')) => {
                self.message = 
                Message::from("ctrl+e to toggle mode, ctrl+s to save, ctrl+q to quit.".to_string());
            },
            // editing mode
            (KeyEventKind::Press, KeyModifiers::CONTROL, KeyCode::Char('e')) if self.editing => {
                self.message = 
                Message::from("arrow keys and pgup/down to navigate.".to_string());
                self.editing = false;
                self.snap_view();
            },
            (KeyEventKind::Press, _, KeyCode::Char(pressed_char)) if self.editing => {
                self.document.append_buffer.count_words();
                if self.document.append_buffer.word_count > 6 {
                    self.save(true);
                } else if self.document.last_edit.elapsed() > Duration::new(5, 0)
                && !self.document.append_buffer.buffer.is_empty() {
                    self.save(false);
                    // self.message = Message::from("buffer saved".to_string());
                }
                self.document.insert(pressed_char);
                self.snap_view();
                self.document.last_edit = Instant::now();
            },
            (KeyEventKind::Press, _, KeyCode::Backspace) if self.editing => {
                // skip checking word count if backspacing
                if self.document.last_edit.elapsed() > Duration::new(5, 0)
                && !self.document.append_buffer.buffer.is_empty() {
                    self.save(false);
                    // self.message = Message::from("buffer saved".to_string());
                }
                if self.position.x > 0 
                || self.position.y > 0 {
                    self.document.delete();
                    self.snap_view();
                };
                self.document.last_edit = Instant::now();
            },
            (KeyEventKind::Press, _, KeyCode::Enter) if self.editing => {
                self.document.append_buffer.count_words();
                if self.document.append_buffer.word_count > 6 {
                    self.save(true);
                } else if self.document.last_edit.elapsed() > Duration::new(5, 0)
                && !self.document.append_buffer.buffer.is_empty() {
                    self.save(false);
                    // self.message = Message::from("buffer saved".to_string());
                }
                self.document.insert('\n');
                self.snap_view();
                self.document.last_edit = Instant::now();
            },
            // viewing mode
            (KeyEventKind::Press, KeyModifiers::CONTROL, KeyCode::Char('e')) if !self.editing => {
                self.editing = true;
                self.snap_view();
            },
            (KeyEventKind::Press, _, KeyCode::Up) if !self.editing => self.viewing_scroll(&ScrollDirection::Up, 1),
            (KeyEventKind::Press, _, KeyCode::Down) if !self.editing => self.viewing_scroll(&ScrollDirection::Down, 1),
            (KeyEventKind::Press, _, KeyCode::PageUp) if !self.editing => self.viewing_scroll(&ScrollDirection::Up, term_height),
            (KeyEventKind::Press, _, KeyCode::PageDown) if !self.editing => self.viewing_scroll(&ScrollDirection::Down, term_height),
            _ => (),
        };
    }
}
