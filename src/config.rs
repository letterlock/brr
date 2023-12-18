use std::fs::File;
use std::io::{
    BufReader, 
    BufRead,
};

use log::error;

pub struct Config {
    pub start_edit: bool, // true = edit, false = view
    pub open_search: bool,
    pub count_on_quit: bool,
    pub quit_times: u8,
    pub save_time: u8,
    pub save_words: u8,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            start_edit: true,
            open_search: true,
            count_on_quit: true,
            quit_times: 2,
            save_time: 5,
            save_words: 6,
        }
    }
}

// BAD: feels like there's a better way to do this.
#[allow(clippy::cast_possible_truncation, clippy::redundant_closure_for_method_calls)]
impl Config {
    pub fn get_config() -> Self {
        if let Ok(config_file) = File::open("brr.conf") {
            let reader = BufReader::new(config_file);
            let mut start_edit = true;
            let mut open_search = true;
            let mut count_on_quit = true;
            let mut quit_times = 2;
            let mut save_time = 5;
            let mut save_words = 6;

            for (line_index, file_line) in reader.lines().enumerate() {
                if let Ok(config_line) = file_line {
                    if !config_line.starts_with('#') {
                        if config_line.contains("start-edit = ") {
                            if config_line.contains("false") {
                                start_edit = false;
                            } else if config_line.contains("true") {
                                start_edit = true;
                            } else {
                                error!("invalid start-edit value. using default.");
                            };
                            continue
                        };
                        if config_line.contains("open-search = ") {
                            if config_line.contains("false") {
                                open_search = false;
                            } else if config_line.contains("true") {
                                open_search = true;
                            } else {
                                error!("invalid open-search value. using default.");
                            };
                            continue
                        };
                        if config_line.contains("count-on-quit = ") {
                            if config_line.contains("false") {
                                count_on_quit = false;
                            } else if config_line.contains("true") {
                                count_on_quit = true;
                            } else {
                                error!("invalid count-on-quit value. using default.");
                            };
                            continue
                        };
                        if config_line.contains("quit-times = ") {
                            if let Some(value) = config_line
                            .chars()
                            .find(|c| c.is_ascii_digit())
                            .and_then(|c| c.to_digit(10)) {
                                quit_times = value as u8;
                            } else {
                                error!("invalid quit-times value. using default.");
                            };
                            continue
                        };
                        if config_line.contains("save-time = ") {
                            if let Some(value) = config_line
                            .chars()
                            .find(|c| c.is_ascii_digit())
                            .and_then(|c| c.to_digit(10)) {
                                save_time = value as u8;
                            } else {
                                error!("invalid save-time value. using default.");
                            };
                            continue
                        };
                        if config_line.contains("save-words = ") {
                            if let Some(value) = config_line
                            .chars()
                            .find(|c| c.is_ascii_digit())
                            .and_then(|c| c.to_digit(10)) {
                                save_words = value as u8;
                            } else {
                                error!("invalid save-words value. using default.");
                            };
                            continue
                        };
                    }
                } else {
                    error!("could not read config file line {}", line_index.saturating_add(1));
                }
            }
            return Self {
                start_edit,
                open_search,
                count_on_quit,
                quit_times,
                save_time,
                save_words,
            }
        }
        error!("could not open config file. using default config.");
        Config::default()
    }
}
