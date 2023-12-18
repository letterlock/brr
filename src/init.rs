use crate::Editor;
use crate::File;
use crate::config::Config;
// use crate::die;

use std::io::stdin;

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
pub struct Init {
    user_input: Option<String>,
    config: Config,
}

impl Init {
    pub fn welcome(mut self, initial_input: Option<String>) {
        // set configs
        self.config = Config::get_config();
        self.user_input = initial_input;

        loop {
            match &self.user_input {
                Some(input) => {
                    if input.as_str() == "-h" 
                    || input.as_str() == "help" {
                        println!("{HELP}");
                        self.get_user_input();
                    } else {
                        let to_open = if self.config.open_search {
                            File::get_file_info(input, true)
                        } else {
                            File::get_file_info(input, false)
                        };
                        Editor::default(to_open, self.config).run();
                        break
                    }
                },
                None => {
                    println!("{WELCOME}");
                    self.get_user_input();
                },
            };
        }
    }

    fn get_user_input(&mut self) {
        let mut input = String::new();

        if let Err(error_msg) = stdin().read_line(&mut input) {
            println!("error: {error_msg}\r{BAD_INPUT}");
            self.user_input = None;
        }
        self.user_input = Some(input.trim().to_string());
    }
}

