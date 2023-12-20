use crate::get_conf_or_log_path;

use std::fs::File;
use std::io::{
    BufReader, 
    BufRead,
};
use crossterm::cursor::SetCursorStyle;

use log::{error, warn};

pub struct Config {
    pub start_edit: bool, // true = edit, false = view
    pub open_search: bool,
    pub count_on_quit: bool,
    pub quit_times: u8,
    pub save_time: u8,
    pub save_words: u8,
    pub cursor_style: SetCursorStyle,
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
            cursor_style: SetCursorStyle::DefaultUserShape,
        }
    }
}

// BAD: feels like there's a more concise way to do this.
// TODO: if the config file is openable and readable, but the individual
// options are mangled somehow, the user should be informed without having
// to open the log file.
#[allow(
    clippy::cast_possible_truncation, // truncating these values shouldn't matter.
    clippy::redundant_closure_for_method_calls, // this seems to be a false positive.
    clippy::too_many_lines, // :^(
)]
impl Config {
    pub fn get_config() -> Self {
        let config_path = get_conf_or_log_path(true);

        if let Some(to_open) = config_path {
            match File::open(to_open) {
                Ok(config_file) => {
                    let reader = BufReader::new(config_file);
                    let mut start_edit = true;
                    let mut open_search = true;
                    let mut count_on_quit = true;
                    let mut quit_times = 2;
                    let mut save_time = 5;
                    let mut save_words = 6;
                    let mut cursor_style = SetCursorStyle::DefaultUserShape;

                    for (line_index, file_line) in reader.lines().enumerate() {
                        if let Ok(config_line) = file_line {
                            if !config_line.starts_with('#') {
                                if config_line.contains("start-edit = ") {
                                    if config_line.contains("false") {
                                        start_edit = false;
                                    } else if config_line.contains("true") {
                                        start_edit = true;
                                    } else {
                                        error!("[config.rs]: invalid start-edit value at line {}. using default.", line_index.saturating_add(1));
                                    };
                                    continue
                                };
                                if config_line.contains("open-search = ") {
                                    if config_line.contains("false") {
                                        open_search = false;
                                    } else if config_line.contains("true") {
                                        open_search = true;
                                    } else {
                                        error!("[config.rs]: invalid open-search value at line {}. using default.", line_index.saturating_add(1));
                                    };
                                    continue
                                };
                                if config_line.contains("count-on-quit = ") {
                                    if config_line.contains("false") {
                                        count_on_quit = false;
                                    } else if config_line.contains("true") {
                                        count_on_quit = true;
                                    } else {
                                        error!("[config.rs]: invalid count-on-quit value at line {}. using default.", line_index.saturating_add(1));
                                    };
                                    continue
                                };
                                if config_line.contains("cursor-style = ") {
                                    if let Some(style) = match_cursor_style(&config_line) {
                                        cursor_style = style;
                                    } else {
                                        error!("[config.rs]: invalid cursor-style value at line {}. using default.", line_index.saturating_add(1));
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
                                        error!("[config.rs]: invalid quit-times value at line {}. using default.", line_index.saturating_add(1));
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
                                        error!("[config.rs]: invalid save-time value at line {}. using default.", line_index.saturating_add(1));
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
                                        error!("[config.rs]: invalid save-words value at line {}. using default.", line_index.saturating_add(1));
                                    };
                                    continue
                                };
                                if !config_line.is_empty() {
                                    warn!("[config.rs]: unknown input on config file line {}", line_index.saturating_add(1));    
                                }
                            };
                        } else {
                            error!("[config.rs]: could not read config file line {}", line_index.saturating_add(1));
                        };
                    };
                    return Self {
                        start_edit,
                        open_search,
                        count_on_quit,
                        quit_times,
                        save_time,
                        save_words,
                        cursor_style,
                    };
                },
                Err(error_msg) => {
                    error!("[config.rs]: {error_msg} - could not open config file. using default config.");
                },
            };
        };
        Config::default()
    }
}

fn match_cursor_style(style: &str) -> Option<SetCursorStyle> {
    match style {
        style if style.contains("default") => Some(SetCursorStyle::DefaultUserShape),
        style if style.contains("blinking-block") => Some(SetCursorStyle::BlinkingBlock),
        style if style.contains("steady-block") => Some(SetCursorStyle::SteadyBlock),
        style if style.contains("blinking-underscore") => Some(SetCursorStyle::BlinkingUnderScore),
        style if style.contains("steady-underscore") => Some(SetCursorStyle::SteadyUnderScore),
        style if style.contains("blinking-bar") => Some(SetCursorStyle::BlinkingBar),
        style if style.contains("steady-bar") => Some(SetCursorStyle::SteadyBar),
        _ => None,
    }
}
