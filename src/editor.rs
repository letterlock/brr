use crate::die;
use crate::Terminal;
use crate::Document;
use crate::File;
use crate::row::DisplayRow;
use crate::Config;

use {
    std::{
        io::Error,
        time::{
            Duration,
            Instant,
        },
        env::consts::OS,
        cmp::PartialEq,
    },
    crossterm::event::{
        Event,
        read,
        poll,
        KeyEvent,
        KeyEventKind,
        KeyModifiers,
        KeyCode,
    },
};

// const VERSION: &str = env!("CARGO_PKG_VERSION");
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

impl Editor {
    pub fn default(file: File, config: Config) -> Self {
        let initial_message = String::from(STANDARD_MESSAGE);
        let mode = if config.start_edit {
            Mode::Edit
        } else {
            Mode::View
        };
        let quit_times = config.quit_times;
        let mut document;

        if file.exists {
            document = Document::open(file);
            Document::wrap_file(&mut document);
            Document::wrap_buffer(&mut document);
        } else {
            document = Document::create(file);
        }
        
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
            die(error_msg);
        };
        self.snap_view();
        loop {
            if let Err(error_msg) = self.refresh_screen() {
                die(error_msg);
            };
            if self.should_quit {
                let written = self.document.written_this_session();
                let quit_msg = if self.config.count_on_quit {
                    format!(
                        "goodbye!\r\napprox. written this session:\r\n  {} words\r\n  {} chars\r\n", 
                        written.words, 
                        written.chars,
                    )
                } else {
                    "goodbye!\r\n".to_string()
                };
                if let Err(error_msg) = Terminal::quit(quit_msg) {
                    die(error_msg);
                };
                break;
            };
            self.process_event();
        };
    }

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

    #[allow(clippy::cast_lossless)]
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
                (KeyModifiers::CONTROL, KeyCode::Char('s')) => self.save(0),
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
                    if self.document.append_buffer.word_count == self.config.save_words as usize {
                        self.save(self.config.save_words);
                    } else if self.document.last_edit.elapsed() > Duration::new(self.config.save_time as u64, 0)
                    && !self.document.append_buffer.buffer.is_empty() {
                        self.save(0);
                    }
                    self.document.insert(pressed_char);
                    self.snap_view();
                    self.document.last_edit = Instant::now();
                },
                (_, KeyCode::Backspace) if self.mode == Mode::Edit => {
                    // skip checking word count if backspacing
                    if self.document.last_edit.elapsed() > Duration::new(self.config.save_time as u64, 0)
                    && !self.document.append_buffer.buffer.is_empty() {
                        self.save(0);
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
                (_, KeyCode::Enter) if self.mode == Mode::Edit => {
                    self.document.append_buffer.count_words();
                    if self.document.append_buffer.word_count == self.config.save_words as usize {
                        self.save(self.config.save_words);
                    } else if self.document.last_edit.elapsed() > Duration::new(self.config.save_time as u64, 0)
                    && !self.document.append_buffer.buffer.is_empty() {
                        self.save(0);
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
    #[allow(clippy::cast_lossless)]
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
                (KeyEventKind::Press, KeyModifiers::CONTROL, KeyCode::Char('s')) => self.save(0),
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
                    if self.document.append_buffer.word_count == self.config.save_words as usize {
                        self.save(self.config.save_words);
                    } else if self.document.last_edit.elapsed() > Duration::new(self.config.save_time as u64, 0)
                    && !self.document.append_buffer.buffer.is_empty() {
                        self.save(0);
                    }
                    self.document.insert(pressed_char);
                    self.snap_view();
                    self.document.last_edit = Instant::now();
                },
                (KeyEventKind::Press, _, KeyCode::Backspace) if self.mode == Mode::Edit => {
                    // skip checking word count if backspacing
                    if self.document.last_edit.elapsed() > Duration::new(self.config.save_time as u64, 0)
                    && !self.document.append_buffer.buffer.is_empty() {
                        self.save(0);
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
                    if self.document.append_buffer.word_count == self.config.save_words as usize {
                        self.save(self.config.save_words);
                    } else if self.document.last_edit.elapsed() > Duration::new(self.config.save_time as u64, 0)
                    && !self.document.append_buffer.buffer.is_empty() {
                        self.save(0);
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
        let max_height = self.document.display_rows.len();
        let last_display_index = self.document.display_rows.len().saturating_sub(1);

        if self.mode == Mode::Edit {
            self.view_pos.y = last_display_index;
            if let Some(last_drow) = self.document.get_display_row(last_display_index) {
                self.view_pos.x = last_drow.len;
            } else {
                self.view_pos.x = 0;
            }
        } else if self.view_pos.y > max_height.saturating_sub((term_height / 2).saturating_add(1)) {
            self.view_pos.y = max_height.saturating_sub((term_height / 2).saturating_add(1));
        };
    }

    fn viewing_scroll(&mut self, direction: &Direction, amount: usize) {
        let max_height = self.document.display_rows.len().saturating_sub(1);
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

        // BAD?: lot of nested ifs here, maybe i can clean this up.
        for term_row in 0..term_height {
            // draw rows in view mode
            if self.mode == Mode::View {
                let index_to_display = self.view_pos.y.saturating_add(term_row);

                if let Some(row_to_render) = self.document.get_display_row(index_to_display) {
                    self.terminal.queue_print(&DisplayRow::render(row_to_render))?;
                } else {
                    self.terminal.queue_print("~")?;
                }
            // draw rows in edit/prompt mode
            } else {
                // if ((y offset + current term row) - editing offset) does not overflow (read:
                // would not become less than zero), check if that row exists in file and print it.
                // otherwise check if that row exists in append buffer and print it. otherwise print ~
                if let Some(index_to_display) = self.view_pos.y.saturating_add(term_row).checked_sub(editing_offset) {
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
            }
            self.terminal.clear_line()?;
            self.terminal.new_line()?;
        }
        Ok(())
    }

    fn draw_status_bar(&mut self) -> Result<(), Error>{
        // BAD: dividing this by three leads to the formatting getting screwed up
        // when the width isn't evenly divisible by three
        let item_width = self.terminal.width / 3;
        let words = self.document.count.words;
        let chars = self.document.count.chars;
        let mut file_name = self.document.file.name.clone();
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
        let count_indicator = format!(
            "{words} words / {chars} chars"
        );

        let file_indicator = format!("{file_name} {dirty_indicator}");

        // BAD?: give some indication if the file name has been truncated?
        file_name.truncate(20);

        let status_bar = format!(
            "{file_indicator:<item_width$}{mode_indicator:^item_width$}{count_indicator:>item_width$}"
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

    pub fn save(&mut self, words: u8) {
        if words > 1 {
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

    pub fn open(&mut self) {
        let prev_mode = self.mode.clone();
        self.mode = Mode::Prompt;
        let input = self.prompt(
            "file name: ", 11, |_, _, _| {}
        ).unwrap_or(None);
        
        if let Some(file_name) = input {
            let file_info = if self.config.open_search {
                File::get_file_info(&file_name, true)
            } else {
                File::get_file_info(&file_name, false)
            };
            let mut document;

            if file_info.exists {
                document = Document::open(file_info);
                Document::wrap_file(&mut document);
                Document::wrap_buffer(&mut document);
            } else {
                document = Document::create(file_info);
            }

            self.document = document;
            self.mode = Mode::Edit;
            self.snap_view();
        } else {
            self.message = Message::from("open aborted".to_string());
            self.mode = prev_mode;
        };
    }

    // BAD: this whole closures and callbacks thing is a bit beyond me
    // so for now i'm just going to hope nothing breaks here
    // too bad! https://doc.rust-lang.org/stable/rust-by-example/fn/closures.html
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
            let event = read()?; // handle this and ideally use fn already in Terminal.rs

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
