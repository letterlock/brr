use crate::{CursorPosition, die};
use std::io::{ self, stdout, Write };
use std::env;
use crossterm::{
    style::{
        Colors,
        SetColors,
        ResetColor,
    },
    event::{ 
        Event,  
        read, 
    }, 
    terminal::{ 
        enable_raw_mode,
        disable_raw_mode,
        size,
        Clear, 
        ClearType,
    },
    ExecutableCommand,
    QueueableCommand,
    cursor::{ 
        MoveTo,
        Hide,
        Show,
        SetCursorStyle,
     },
};

pub struct Terminal {
    pub term_size: TermSize,
}

pub struct TermSize {
    pub width: u16,
    pub height: u16,
}

impl Terminal {
    pub fn default() -> Self {
        // fix error handling here
        let term_size = size().unwrap();
        if let Err(error_msg) = enable_raw_mode() {
            die(&error_msg);
        };

        Self {
            term_size: TermSize {
                width: term_size.0,
                height: term_size.1.saturating_sub(2),
            }
        }
    }

    pub fn size(&self) -> &TermSize {
        &self.term_size
    }

    pub fn quit() {
        Terminal::reset_colors();
        Terminal::clear_screen();
        if let Err(error_msg) = disable_raw_mode() {
            println!("could not disable raw mode: {error_msg}\r");
        } else {
            println!("goodbye\r");
        };
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn cursor_position(position: &CursorPosition) {
        let CursorPosition{mut cursor_x, mut cursor_y} = position;
        cursor_x = cursor_x.saturating_add(1);
        cursor_y = cursor_y.saturating_add(1);
        let x = cursor_x as u16;
        let y = cursor_y as u16;
        stdout().queue(MoveTo(x.saturating_sub(1), y.saturating_sub(1))).ok();
    }

    pub fn flush() -> Result<(), std::io::Error> {
        io::stdout().flush()
    }

    pub fn read_event(&mut self) -> Result<Event, std::io::Error> {
        // below was once enclosed in a loop that is currently redundant
        // just noting this in case it becomes relevant later
        let event = read();
    
        if let Ok(Event::Resize(new_width, new_height)) = event {
            self.term_size.width = new_width;
            if env::consts::OS == "windows" {
                self.term_size.height = new_height.saturating_sub(1);
            } else {
                self.term_size.height = new_height.saturating_sub(2);
            }
        };
        
        event
    }

    pub fn cursor_hide() {
        stdout().execute(Hide).ok();
    }

    pub fn cursor_show() {
        stdout().execute(Show).ok();
    }

    pub fn cursor_style() {
        stdout().execute(SetCursorStyle::BlinkingBlock).ok();
    }
    
    pub fn clear_screen() {
        stdout().execute(Clear(ClearType::All)).ok();
    }

    pub fn clear_current_line() {
        stdout().execute(Clear(ClearType::CurrentLine)).ok();
    }

    pub fn set_colors(colors: Colors) {
        stdout().execute(SetColors(colors)).ok();
    }

    pub fn reset_colors() {
        stdout().execute(ResetColor).ok();
    }
}
