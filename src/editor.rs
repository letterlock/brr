use crate::Document;
use crate::Row;
use crate::Terminal;
use crate::die;
use std::env;
use core::time::Duration;
use std::time::Instant;
use crossterm::{
    style::{ Colors, Color },
    event::{ 
        read,
        Event, 
        KeyCode, 
        KeyModifiers,
        KeyEvent,
    },
};

const VERSION: &str = env!("CARGO_PKG_VERSION"); // currently only for welcome message
const STATUS_FG_COLOR: Color = Color::Rgb{r: 63, g: 63, b: 63};
const STATUS_BG_COLOR: Color = Color::Rgb{r: 239, g: 239, b: 239};
const QUIT_TIMES: u8 = 3;

#[derive(PartialEq, Clone, Copy)]
pub enum SearchDirection {
    Forward,
    Backward,
}

#[derive(Default, Clone)]
pub struct CursorPosition {
    pub cursor_x: usize,
    pub cursor_y: usize,
}

struct StatusMessage {
    text: String,
    time: Instant,
}

impl StatusMessage {
    fn from(message: String) -> Self {
        Self {
            text: message,
            time: Instant::now(),
        }
    }
}

pub struct Editor {
    should_quit: bool,
    terminal: Terminal,
    cursor_position: CursorPosition,
    offset: CursorPosition,
    document: Document,
    status_message: StatusMessage,
    quit_times: u8,
}

impl Editor {
    pub fn run(&mut self) {
        Terminal::cursor_style(); // this could be a setting at some point
        self.cursor_to_end();
        loop {
            if let Err(error_msg) = self.refresh_screen() {
                die(&error_msg);
            }
            if self.should_quit {
                break;
            }
            if let Err(error_msg) = self.process_keypress() {
                die(&error_msg);
            }
        }
    }

    pub fn default() -> Self {
        let args: Vec<String> = env::args().collect();
        let mut initial_status = 
            String::from("help: ctrl-f to find | ctrl-s to save | ctrl-q to quit");
        let document = if let Some(file_name) = args.get(1) {
            let doc = Document::open(file_name);

            if let Ok(doc) = doc {
                doc
            } else {
                initial_status = format!("err: could not open file: {file_name}");
                Document::default()
            }
        } else {
            Document::default()
        };

        Self { 
            should_quit: false,
            terminal: Terminal::default(),
            cursor_position: CursorPosition::default(),
            document,
            offset: CursorPosition::default(),
            status_message: StatusMessage::from(initial_status),
            quit_times: QUIT_TIMES,
        }
    }

    fn refresh_screen(&mut self) -> Result<(), std::io::Error> {
        Terminal::cursor_hide();
        Terminal::cursor_position(&CursorPosition::default());
        if self.should_quit {
            Terminal::quit();
        } else {
            self.draw_rows();
            self.draw_status_bar();
            self.draw_message_bar();
            Terminal::cursor_position(&CursorPosition {
                cursor_x: self.cursor_position.cursor_x.saturating_sub(self.offset.cursor_x),
                cursor_y: self.cursor_position.cursor_y.saturating_sub(self.offset.cursor_y),
            });
        }
        Terminal::cursor_show();
        Terminal::flush()
    }

    pub fn cursor_to_end(&mut self) {
        let end_y = self.document.len().saturating_sub(1);
        let end_x = if let Some(row) = self.document.row(end_y) {
            row.len()
        } else {
            0
        };

        
        self.cursor_position = CursorPosition {
            cursor_x: end_x,
            cursor_y: end_y,
        };
        self.scroll();
    }

    fn draw_status_bar(&self) {
        let mut status_msg;
        let term_width = self.terminal.size().width as usize;
        let dirty_indicator = if self.document.is_dirty() {
            "(*)"
        } else {
            ""
        };
        let mut file_name = "[no name]".to_owned();

        if let Some(name) = &self.document.file_name {
            file_name = name.clone();
            file_name.truncate(20);
        }

        status_msg = format!(
            "{} - {} lines {}", 
            file_name, 
            self.document.len(),
            dirty_indicator
        );

        let line_indicator = format!(
            "{} / {}",
            self.cursor_position.cursor_y.saturating_add(1),
            self.document.len()
        );

        let status_len = status_msg.len().saturating_add(line_indicator.len());

        status_msg.push_str(&" ".repeat(term_width.saturating_sub(status_len)));
        status_msg = format!("{status_msg}{line_indicator}");
        status_msg.truncate(term_width);

        Terminal::set_colors(Colors::new(
            STATUS_FG_COLOR,
            STATUS_BG_COLOR
        ));
        println!("{status_msg}\r");
        Terminal::reset_colors();
    }

    fn draw_message_bar(&self) {
        Terminal::clear_current_line();
        let message = &self.status_message;
        
        if message.time.elapsed() < Duration::new(5, 0) {
            let mut text = message.text.clone();
            text.truncate(self.terminal.size().width as usize);
            print!("{text}");
        }
    }

    fn save (&mut self) {
        if self.document.file_name.is_none() {
            // fix error handling here
            let new_name = self.prompt(
                "save as: ", |_, _, _| {}).unwrap_or(None);
            if new_name.is_none() {
                self.status_message = StatusMessage::from("save aborted".to_owned());
                return;
            };
            self.document.file_name = new_name;
        }
        if self.document.save().is_ok() {
            self.status_message = 
                StatusMessage::from("file saved successfully".to_owned());
        } else {
            self.status_message =
                StatusMessage::from("error writing file".to_owned());
        }
    }

    fn search(&mut self) {
        let old_position = self.cursor_position.clone();
        let mut direction = SearchDirection::Forward;
        // todo: i hate this code block and i'd really like to refactor it to
        // something actually readable at some point
        let query = self.prompt(
            "search (esc to cancel, arrows to navigate): ",
            |editor, key, query| {
                let mut moved = false;
                
                match key.code {
                    KeyCode::Right 
                    | KeyCode::Down => {
                        direction = SearchDirection::Forward;
                        editor.move_cursor(KeyCode::Right);
                        moved = true;
                    },
                    KeyCode::Left
                    | KeyCode::Up => direction = SearchDirection::Backward,
                    _ => direction = SearchDirection::Forward,
                };
                if let Some(position) = 
                editor.document.find(query, &editor.cursor_position, direction) {
                    editor.cursor_position = position;
                    editor.scroll();
                } else if moved {
                    editor.move_cursor(KeyCode::Left);
                };
            },
        ).unwrap_or(None); 

        if query.is_none() {
            self.cursor_position = old_position;
            self.scroll();
        }
    }

    fn process_keypress(&mut self) -> Result<(), std::io::Error> {
        let event = Terminal::read_event(&mut self.terminal)?;

        if let Event::Key(pressed_key) = event {
            match (pressed_key.modifiers, pressed_key.code) {
                (KeyModifiers::CONTROL, KeyCode::Char('q')) => {
                    if self.quit_times > 0 && self.document.is_dirty() {
                        self.status_message = StatusMessage::from(format!(
                            "file has unsaved changes. press ctrl-q {} more times to quit anyway.",
                            self.quit_times
                        ));
                        self.quit_times -= 1;
                        return Ok(());
                    }
                    self.should_quit = true;
                },
                (KeyModifiers::CONTROL, KeyCode::Char('s')) => self.save(),
                (KeyModifiers::CONTROL, KeyCode::Char('f')) => self.search(),
                (_, KeyCode::Char(character)) => {
                    self.document.insert(&self.cursor_position, character);
                    self.move_cursor(KeyCode::Right);
                },
                (_, KeyCode::Delete) => self.document.delete(&self.cursor_position),
                (_, KeyCode::Backspace) => {
                    if self.cursor_position.cursor_x > 0 || self.cursor_position.cursor_y > 0 {
                        self.move_cursor(KeyCode::Left);
                        self.document.delete(&self.cursor_position);
                    }
                },
                (_, KeyCode::Enter) => {
                    self.document.insert(&self.cursor_position, '\n');
                    self.move_cursor(KeyCode::Right);
                }
                (_, KeyCode::Up
                | KeyCode::Down
                | KeyCode::Left
                | KeyCode::Right
                | KeyCode::PageUp
                | KeyCode::PageDown
                | KeyCode::End
                | KeyCode::Home) => self.move_cursor(pressed_key.code), 
                _ => (),
            }
        } 
        self.scroll();
        if self.quit_times < QUIT_TIMES {
            self.quit_times = QUIT_TIMES;
            self.status_message = StatusMessage::from(String::new());
        }
        // fix error handling here
        Ok(())
    }

    fn scroll(&mut self) {
        let CursorPosition { cursor_x, cursor_y } = self.cursor_position;
        let terminal_width = self.terminal.size().width as usize;
        let terminal_height = self.terminal.size().height as usize;
        let mut offset = &mut self.offset;

        if cursor_y < offset.cursor_y {
            offset.cursor_y = cursor_y;
        } else if cursor_y >= offset.cursor_y.saturating_add(terminal_height) {
            offset.cursor_y = cursor_y.saturating_sub(terminal_height).saturating_add(1);
        }
        if cursor_x < offset.cursor_x {
            offset.cursor_x = cursor_x;
        } else if cursor_x >= offset.cursor_x.saturating_add(terminal_width) {
            offset.cursor_x = cursor_x.saturating_sub(terminal_width).saturating_add(1);
        }
    }

    fn move_cursor(&mut self, direction: KeyCode) {
        let terminal_height = self.terminal.size().height as usize;
        let CursorPosition { mut cursor_x, mut cursor_y } = self.cursor_position;
        let max_height = self.document.len();
        let mut max_width = if let Some(row) = self.document.row(cursor_y) {
            row.len()
        } else {
            0
        };
        
        match direction {
            KeyCode::Up => cursor_y = cursor_y.saturating_sub(1),
            KeyCode::Down => {
                if cursor_y < max_height {
                    cursor_y = cursor_y.saturating_add(1);
                }
            },
            KeyCode::Left => {
                if cursor_x > 0 {
                    cursor_x -= 1;
                } else if cursor_y > 0 {
                    cursor_y -= 1;
                    if let Some(row) = self.document.row(cursor_y) {
                        cursor_x = row.len();
                    } else {
                        cursor_x = 0;
                    }
                }
            },
            KeyCode::Right => {
                if cursor_x < max_width {
                    cursor_x += 1;
                } else if cursor_y < max_height {
                    cursor_y += 1;
                    cursor_x = 0;
                }
            },
            KeyCode::PageUp => {
                cursor_y = if cursor_y > terminal_height {
                    cursor_y.saturating_sub(terminal_height)
                } else {
                    0
                }
            },
            KeyCode::PageDown => {
                cursor_y = if cursor_y < terminal_height {
                    cursor_y.saturating_add(terminal_height)
                } else {
                    max_height
                }
            },
            KeyCode::Home => cursor_x = 0,
            KeyCode::End => cursor_x = max_width,
            _ => (),
        }

        max_width = if let Some(row) = self.document.row(cursor_y) {
            row.len()
        } else {
            0
        };

        if cursor_x > max_width {
            cursor_x = max_width;
        }

        self.cursor_position = CursorPosition { cursor_x, cursor_y }
    }

    fn draw_welcome_message(&self) {
        let mut welcome_message = format!(" brr -- {VERSION} \r");
        let width = self.terminal.size().width as usize;
        let welcome_msg_length = welcome_message.len();
        let padding = width.saturating_sub(welcome_msg_length) / 2;
        let spaces = " ".repeat(padding.saturating_sub(1));
        welcome_message = format!("~{spaces}{welcome_message}");
        welcome_message.truncate(width);
        println!("{welcome_message}\r");
    }
    
    pub fn draw_row(&self, row: &Row) {
        let width = self.terminal.size().width as usize;
        let start = self.offset.cursor_x;
        let end = self.offset.cursor_x.saturating_add(width);
        let row = row.render(start, end);

        println!("{row}\r");
    }

    fn draw_rows(&self) {
        let height = self.terminal.size().height;
        
        for terminal_row in 0..height {
            Terminal::clear_current_line();
            if let Some(row) = self
                .document
                .row(self.offset.cursor_y.saturating_add(terminal_row as usize)) 
            {
                self.draw_row(row);
            } else if self.document.is_empty() && terminal_row == height / 3 {
                self.draw_welcome_message();
            } else {
                println!("~\r");
            }
        }
    }

    // this whole closures and callbacks thing is a bit beyond me
    // so for now i'm just going to hope nothing breaks here
    // too bad!
    fn prompt<C>(&mut self, prompt: &str, mut callback: C) -> Result<Option<String>, std::io::Error> 
    where
        C: FnMut(&mut Self, KeyEvent, &String),
    {
        let mut user_input = String::new();

        loop {
            self.status_message = StatusMessage::from(format!("{prompt}{user_input}"));
            self.refresh_screen()?;
            let event = read().unwrap(); // handle this and ideally use fn already in Terminal.rs

            if let Event::Key(key) = event {
                match key.code {
                    KeyCode::Backspace => user_input.truncate(user_input.len().saturating_sub(1)),
                    KeyCode::Enter => break,
                    KeyCode::Char(character) => {
                        if !character.is_control() {
                            user_input.push(character);
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
        self.status_message = StatusMessage::from(String::new());
        
        if user_input.is_empty() {
            return Ok(None);
        }
        // fix error handling here
        Ok(Some(user_input))
    }
}