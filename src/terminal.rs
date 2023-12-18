use crate::{Position, die::die};

use crossterm::{
    terminal::{
        disable_raw_mode,
        enable_raw_mode,
        Clear,
        ClearType,
        size, LeaveAlternateScreen, EnterAlternateScreen,
    },
    cursor::{
        MoveToNextLine,
        MoveTo,
        Hide,
        Show,
    },
    style::{
        Print,
        SetAttribute,
        Attribute::{
            Reverse,
            NoReverse,
        },
    },
    ExecutableCommand, 
    QueueableCommand,
};
use std::io::{
    Write,
    Stdout,
    stdout,
    Error,
};

pub struct Terminal {
    pub stdout: Stdout,
    pub width: usize,
    pub height: usize,
}

impl Terminal {
    pub fn default() -> Self {
        let (columns, rows) = Terminal::get_term_size();

        Self {
            stdout: stdout(),
            width: columns,
            height: rows.saturating_sub(2),
        }
    }

    pub fn get_term_size() -> (usize, usize) {
        match size() {
            Ok(size) => (size.0 as usize, size.1 as usize),
            Err(error_msg) => {
                die(error_msg);
                (0, 0)
            },
        }
    }

    pub fn init() -> Result<(), Error> {
        stdout().execute(EnterAlternateScreen)?;
        enable_raw_mode()?;
        Ok(())
    }

    pub fn quit(quit_msg: String) -> Result<(), Error> {
        stdout().queue(Hide)?;
        stdout().queue(MoveTo(0, 0))?;
        stdout().queue(Clear(ClearType::All))?;
        stdout().queue(Show)?;
        stdout().queue(LeaveAlternateScreen)?;
        stdout().queue(Print(quit_msg))?;
        disable_raw_mode()?;
        stdout().flush()?;
        Ok(())
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn move_cursor(&mut self, position: &Position) -> Result<(), Error> {
        let Position{mut x, mut y} = position;
        // add 1 to change from 0-indexing to the 1-indexing that
        // terminals use
        x = x.saturating_add(1);
        y = y.saturating_add(1);
        let new_x = x;
        let new_y = y;
        self.stdout.queue(MoveTo(
            new_x.saturating_sub(1) as u16,
            new_y.saturating_sub(1) as u16
        ))?;
        Ok(())
    }

    pub fn clear_line(&mut self) -> Result<(), Error> {
        self.stdout.queue(Clear(ClearType::UntilNewLine))?;
        Ok(())
    }

    pub fn new_line(&mut self) -> Result<(), Error> {
        self.stdout.queue(MoveToNextLine(1))?;
        Ok(())
    }

    pub fn queue_print(&mut self, to_print: &str) -> Result<(), Error> {
        self.stdout.queue(Print(to_print))?;
        Ok(())
    }

    pub fn cursor_hide(&mut self) -> Result<(), Error> {
        self.stdout.queue(Hide)?;
        Ok(())
    }

    pub fn cursor_show(&mut self) -> Result<(), Error> {
        self.stdout.queue(Show)?;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), Error> {
        self.stdout.flush()?;
        Ok(())
    }

    pub fn reverse_colors(&mut self) -> Result<(), Error>{
        self.stdout.queue(SetAttribute(Reverse))?;
        Ok(())
    }

    pub fn no_reverse_colors(&mut self) -> Result<(), Error>{
        self.stdout.queue(SetAttribute(NoReverse))?;
        Ok(())
    }

    pub fn clear_all(&mut self) -> Result<(), Error> {
        self.stdout.execute(Clear(ClearType::All))?;
        Ok(())
    }

    pub fn cursor_hide_now(&mut self) -> Result<(), Error> {
        self.stdout.execute(Hide)?;
        Ok(())
    }
}
