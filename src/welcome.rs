use crate::Editor;
use crate::die;

use std::io::Error;
use std::fs::read_dir;
use std::path::PathBuf;
use std::{
    path::Path,
    io::stdin,
};

const WELCOME: &str = "\r
welcome to\r
  ______                \r
  ___  /________________\r
  __  __ -_  ___/_  ___/\r
  _  /_/ /  /   _  /    \r
  /_.___//_/    /_/     \r
                        \r
    the perfunctory prose proliferator\r
\r
  please specify a file name, type 'help' for help, or press ctrl+c to exit.";
// BAD: this might suggest that the user should type brr again if they use the help prompt
const HELP: &str = "brr help:\r
  -> usage: brr [OPTIONS/COMMANDS] [FILENAME]\r
  -h option or 'help' command prints this dialogue.";
const BAD_INPUT: &str = "-> usage: brr [OPTIONS/COMMANDS] [FILENAME]\r
  use option '-h' or command 'help' for help.";

#[derive(Default)]
pub struct Welcome {
    user_input: Option<String>,
    current_dir: PathBuf,
}

enum Extension {
    Md,
    Txt,
    Idk,
}

impl Welcome {
    pub fn welcome(&mut self, current_dir: PathBuf) {
        self.user_input = std::env::args().nth(1);
        self.current_dir = current_dir;
        let mut file_name = String::new();
        
        loop {
            if let Some(input) = &self.user_input {
                match input.as_str() {
                    "-h"
                    | "help" => {
                        println!("{HELP}");
                        self.get_user_input();
                    },
                    _ => {
                        if Path::new(&input).extension().is_some() {
                            file_name.push_str(input);
                        } else {
                            match self.file_in_current_dir(input) {
                                Ok(Extension::Md) => {
                                    file_name.push_str(input);
                                    file_name.push_str(".md");
                                    break;
                                },
                                Ok(Extension::Txt) => {
                                    file_name.push_str(input);
                                    file_name.push_str(".txt");
                                    break;
                                },
                                // BAD: fix this ugliness
                                Ok(Extension::Idk) => (),
                                Err(error_msg) => die(error_msg),
                            };
                        }
                    },
                };
            } else {
                println!("{WELCOME}");
                self.get_user_input();
            };
        }
        Editor::default(&file_name).run();
    }

    fn get_user_input(&mut self) {
        let mut input = String::new();

        if let Err(error_msg) = stdin().read_line(&mut input) {
            println!("error: {error_msg}\r{BAD_INPUT}");
            self.user_input = None;
        }
        self.user_input = Some(input.trim().to_string());
    }

    // searches the current directory for a file matching the
    // args but with a .txt or .md extension
    fn file_in_current_dir(&self, file_name: &str) -> Result<Extension, Error> {
        let current_dir = self.current_dir.as_path();
        
        for entry in read_dir(current_dir)? {
            let item = entry?;
            let contains_name = item.file_name().to_str().is_some_and(|item| item.contains(file_name));
            let is_md = item.file_name().to_str().is_some_and(|item| item.contains(".md"));
            let is_txt = item.file_name().to_str().is_some_and(|item| item.contains(".txt"));
            
            if contains_name 
            && is_md {
                return Ok(Extension::Md);
            } else if contains_name
            && is_txt {
                return Ok(Extension::Txt);
            };
        };
        Ok(Extension::Idk)
    }
}

