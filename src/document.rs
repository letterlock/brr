use crate::CursorPosition;
use crate::Row;
use crate::SearchDirection;
use std::fs;
use std::io::{Error, Write};

#[derive(Default)]
pub struct Document {
    rows: Vec<Row>,
    pub file_name: Option<String>,
    is_dirty: bool,
}

impl Document {
    pub fn open(file_name: &str) -> Result<Self, std::io::Error> {
        let contents = fs::read_to_string(file_name)?;
        let mut rows = Vec::new();

        for value in contents.lines() {
            rows.push(Row::from(value));
        }

        Ok(Self { 
            rows,
            file_name: Some(file_name.to_owned()),
            is_dirty: false,
        })
        
    }
    
    pub fn row(&self, index: usize) -> Option<&Row> {
        self.rows.get(index)
    }
    
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn insert_newline(&mut self, at: &CursorPosition) {
        if at.cursor_y > self.rows.len() {
            return;
        }
        if at.cursor_y == self.rows.len() {
            self.rows.push(Row::default());
            return;        
        }
        let new_row = self.rows[at.cursor_y].split(at.cursor_x);
        
        self.rows.insert(at.cursor_y.saturating_add(1), new_row);

    }

    pub fn insert(&mut self, at: &CursorPosition, character: char) {
        if at.cursor_y > self.rows.len() {
            return;
        }

        self.is_dirty = true;
        
        if character == '\n' {
            self.insert_newline(at);
            return;
        }
        if at.cursor_y == self.rows.len() {
            let mut row = Row::default();
            row.insert(0, character);
            self.rows.push(row);
        } else {
            let row = &mut self.rows[at.cursor_y];
            row.insert(at.cursor_x, character);
        }
    }

    pub fn delete(&mut self, at: &CursorPosition) {
        let len = self.rows.len();

        if at.cursor_y >= len {
            return;
        }

        self.is_dirty = true;

        if at.cursor_x == self.rows[at.cursor_y].len() && at.cursor_y.saturating_add(1) < len {
            let next_row = self.rows.remove(at.cursor_y.saturating_add(1) + 1);
            let row = &mut self.rows[at.cursor_y];
            
            row.append(&next_row);
        } else {
            let row = &mut self.rows[at.cursor_y];
            
            row.delete(at.cursor_x);
        }
    }

    pub fn save(&mut self) -> Result<(), Error> {
        if let Some(file_name) = &self.file_name {
            let mut file = fs::File::create(file_name)?;
            
            for row in &self.rows {
                file.write_all(row.as_bytes())?;
                file.write_all(b"\n")?;
            }
            self.is_dirty = false;
        }
        Ok(())
    }

    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    pub fn find(&self, query: &str, at: &CursorPosition, direction: SearchDirection) -> Option<CursorPosition> {
        if at.cursor_y >= self.rows.len() {
            return None;
        };
        
        let mut position = CursorPosition { cursor_x: at.cursor_x, cursor_y: at.cursor_y};
        let start = if direction == SearchDirection::Forward {
            at.cursor_y
        } else {
            0
        };
        let end = if direction == SearchDirection::Forward {
            self.rows.len()
        } else {
            at.cursor_y.saturating_add(1)
        };
        
        for _ in start..end {
            if let Some(row) = self.rows.get(position.cursor_y) {
                if let Some(x) = row.find(query, position.cursor_x, direction) {
                    position.cursor_x = x;
                    return Some(position);
                }
                if direction == SearchDirection::Forward {
                    position.cursor_y = position.cursor_y.saturating_add(1);
                    position.cursor_x = 0;
                } else {
                    position.cursor_y = position.cursor_y.saturating_sub(1);
                    position.cursor_x = self.rows[position.cursor_y].len();
                }
            } else {
                return None;
            }
        }
        None
    }
}