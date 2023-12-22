use crate::{Editor, Metadata, Config};
use std::io::{Error, Write, stdin, stdout};

// -----------------

const VERSION: &str = env!("CARGO_PKG_VERSION");
const WELCOME: &str = "\r
  welcome to\r
      ▀██               \r
     ▀ ██ ▄▄▄           \r
    ▀▀ ██▀  ██ █▄▀▀ █▄▀▀\r
   ▀▀▀ ██    █ █    █   \r
  ▀▀▀▀ ▀█▄▄▄▀  ▀    ▀   \r
    the perfunctory prose proliferator\r
\r
please specify a file name, type 'help' for help, or press ctrl+c to exit.";
const PROMPT: &str = " > ";
// BAD: this might suggest that the user should type brr again if they use the help prompt
const HELP: &str = "brr help:\r
  -> usage: brr [OPTIONS/COMMANDS] [FILENAME]\r
  \r
  -h / help    - print help\r
  -v / version - print version";

#[derive(Default)]
pub struct Init {
    user_input: Option<String>,
    config: Config,
}

impl Init {
    pub fn welcome(mut self, initial_input: Option<String>) -> Result<(), Error>{
        // set configs
        self.config = Config::get_config();
        self.user_input = initial_input;
        // this is so that we can just quit if options are
        // called and we're not within the dialogue loop.
        let mut exit_after = true;

        loop {
            if let Some(input) = &self.user_input {
                match input.as_str() {
                    "v" 
                    | "version" => {
                        println!("brr -- version {VERSION}");
                        if exit_after {
                            break
                        };
                        print!("{PROMPT}");
                        stdout().flush()?;
                        self.get_user_input();
                    },
                    "-h" 
                    | "help" => {
                        println!("{HELP}");
                        if exit_after {
                            break
                        };
                        print!("{PROMPT}");
                        stdout().flush()?;
                        self.get_user_input();
                    },
                    _ => {
                        let to_open = if self.config.open_search {
                            Metadata::get_file_info(input, true)
                        } else {
                            Metadata::get_file_info(input, false)
                        };
                        Editor::default(to_open, self.config).run();
                        break
                    },
                };
            } else {
                println!("{WELCOME}");
                print!("{PROMPT}");
                stdout().flush()?;
                self.get_user_input();
                exit_after = false;
            };
        }
        Ok(())
    }

    fn get_user_input(&mut self) {
        let mut input = String::new();

        if let Err(error_msg) = stdin().read_line(&mut input) {
            println!("error: {error_msg} - bad input.");
            self.user_input = None;
        }
        self.user_input = Some(input.trim().to_string());
    }
}
