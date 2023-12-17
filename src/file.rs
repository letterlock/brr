use crate::die::die;

use std::{
    path::{
        Path,
        PathBuf,
    },
    fs::{
        read_dir,
        read_to_string,
    },
    env::current_dir,
};

pub struct File {
    pub current_dir: PathBuf,
    pub name: String,
    pub extension: Extension,
    pub exists: bool,
    pub as_string: String,
}

#[derive(PartialEq)]
pub enum Extension {
    Md,
    Txt,
    Other,
    None,
}

// BAD: this doesn't behave intuitively if there is a both a
// [file_name].md and [file_name].txt that match
impl File {
    pub fn get_file_info(user_input: &str) -> Self {
        let mut current_directory = PathBuf::new();
        let mut name = String::new();
        let mut exists = false;
        let mut extension = File::get_extension(user_input);
        let mut as_string = String::new();

        if !user_input.is_empty() {
            name.push_str(user_input);
            // current_directory = current_dir();
            match current_dir() {
                // BAD: this should do something more explicit than just die.
                Err(error_msg) => die(error_msg),
                Ok(dir) => current_directory = dir,
            };

            // if no extension, search the current directory for
            // a file matching name but with a .txt or .md extension
            if extension == Extension::None {
                let dir = read_dir(&current_directory).ok();

                if let Some(dir_iter) = dir {
                    for item in dir_iter.flatten() {
                        let contains_name = item.file_name().to_str().is_some_and(|item_name| item_name.contains(&name));
                        let is_md = item.file_name().to_str().is_some_and(|item_name| item_name.contains(".md"));
                        let is_txt = item.file_name().to_str().is_some_and(|item_name| item_name.contains(".txt"));
    
                        if contains_name 
                        && is_md {
                            extension = Extension::Md;
                            name.push_str(".md");
                        } else if contains_name
                        && is_txt {
                            extension = Extension::Txt;
                            name.push_str(".txt");
                        };
                    };
                };
            };
            // BAD?: see https://doc.rust-lang.org/std/path/struct.Path.html#method.try_exists
            // if the user input is a path that exists, set exists to true
            if Path::new(&name).exists() {
                exists = true;
                match read_to_string(&name) {
                    Ok(file_string) => as_string = file_string,
                    Err(error_msg) => die(error_msg),
                };
            };
        }

        Self { 
            current_dir: current_directory,
            name,
            extension,
            exists,
            as_string,
        }
    }

    

    // gets the extension (if any) from self.file_name
    // and propogates self.extension, specifying if the
    // extension is .md or .txt
    pub fn get_extension(file_name: &str) -> Extension {
        let file_path = Path::new(&file_name);

        if let Some(extension) = file_path.extension() {
            match extension.to_str() {
                Some("md") => return Extension::Md,
                Some("txt") => return Extension::Txt,
                Some(_) => return Extension::Other,
                None => return Extension::None,
            }
        }
        Extension::None
    }

    // pub fn get_current_dir() -> PathBuf {
    //     let mut current = PathBuf::new();

    //     match current_dir() {
    //         // BAD: this should do something more explicit than just die.
    //         Err(error_msg) => die(error_msg),
    //         Ok(dir) => current = dir,
    //     };
    //     current
    // }
}