use {
    std::{
        path::{Path, PathBuf},
        fs::{read_dir, create_dir_all},
        env::{current_dir, current_exe, var, consts::OS},
    },
    log::{error, info, trace, warn},
};

// -----------------

pub struct Metadata {
    pub current_dir: PathBuf,
    pub path: PathBuf,
    pub name: String,
    pub extension: Extension,
    // pub exists: bool,
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
impl Metadata {
    pub fn get_file_info(user_input: &str, search_ext: bool) -> Self {
        let mut path = PathBuf::from(&user_input);
        let mut name = user_input.to_string();
        // let mut exists = path.is_file();
        let mut extension = Metadata::get_extension(&path);
        let current_directory = match current_dir() {
            Err(error_msg) => {
                error!("[metadata.rs]: {error_msg} - could not get current directory.");
                PathBuf::new()
            },
            Ok(dir) => dir,
        };

        if user_input.is_empty() {
            info!("user input looks empty. going to create a new file.");
        } else {
            if path.is_absolute() {
                // could put in more checking here if i want to get more granular information later.
                if let Some(os_str) = path.file_name() {
                    if let Some(name_from_path) = os_str.to_str() {
                        name = name_from_path.to_string();
                    } else {
                        error!("[metadata.rs]: could not convert OsStr, is file path a valid UTF-8 string?");
                    };
                } else {
                    error!("[metadata.rs]: could not get file name from path, does path terminate in '..'?");
                };
                if extension == Extension::None 
                && search_ext {
                    if let Some(parent) = path.parent() {
                        let parent = parent.to_path_buf();
                        let (path_result, name_result, ext_result) = Metadata::search_ext(&parent, &name);

                        if ext_result == Extension::None {
                            info!("extension search yielded no results.");
                        } else {
                            path = path_result;
                            name = name_result;
                            extension = ext_result;
                            // exists = true;
                        };
                    };
                } else {
                    info!("could not get parent directory. input path has no parent or parent cannot be accessed.");
                };
            } else {
                info!("input path is either not absolute or not a file. full path and file name will be set to input.");
            };

            // if no extension, search the current directory for
            // a file matching name but with a .txt or .md extension.
            // set extension based on either the parent of the input directory
            // or the current directory.
            if extension == Extension::None 
            && search_ext {
                let (path_result, name_result, ext_result) = Metadata::search_ext(&current_directory, &name);

                if ext_result == Extension::None {
                    info!("extension search yielded no results.");
                } else {
                    path = path_result;
                    name = name_result;
                    extension = ext_result;
                    // exists = true;
                };
            };
        };

        Self { 
            current_dir: current_directory,
            path,
            name,
            extension,
            // exists,
        }
    }

    // gets the extension (if any) from self.file_name
    // and propogates self.extension, specifying if the
    // extension is .md or .txt
    pub fn get_extension(path: &Path) -> Extension {
        if let Some(extension) = path.extension() {
            match extension.to_str() {
                Some("md") => return Extension::Md,
                Some("txt") => return Extension::Txt,
                Some(_) => return Extension::Other,
                None => return Extension::None,
            }
        }
        Extension::None
    }

    pub fn search_ext(path: &PathBuf, name: &str) -> (PathBuf, String, Extension) {
        let to_search = read_dir(path).ok();
        let mut path = path.clone();
        let mut name = name.to_string();

        if let Some(search_iter) = to_search {
            for item in search_iter.flatten() {
                let contains_name = item
                .file_name()
                .to_str()
                .is_some_and(|item_name| item_name.contains(&name));
                let is_md = item
                .file_name()
                .to_str()
                .is_some_and(|item_name| item_name.contains(".md"));
                let is_txt = item
                .file_name()
                .to_str()
                .is_some_and(|item_name| item_name.contains(".txt"));

                if contains_name 
                && is_md {
                    name.push_str(".md");
                    path.push(PathBuf::from(&name));
                    return (path, name, Extension::Md);
                } else if contains_name
                && is_txt {
                    name.push_str(".txt");
                    path.push(PathBuf::from(&name));
                    return (path, name, Extension::Txt);
                };
            };
        };
        (path, name, Extension::None)
    }
}

// attempts to get config or log file path in the
// folder of the brr executable pass true for 
// config path or false for log path.
#[allow(clippy::needless_return)] // seems to be a false positive
pub fn get_conf_or_log_path(config: bool) -> Option<PathBuf> {
    let config_key = "XDG_CONFIG_HOME";
    let state_key = "XDG_STATE_HOME";
    let home_key = "HOME";

    if config && OS == "linux" {
        if let Ok(config_dir) = var(config_key) {
            trace!("found $XDG_CONFIG_HOME: {config_dir}");
            let mut config_path = PathBuf::from(config_dir);
            config_path.push("brr/brr.conf");
            info!("using config path: {}", config_path.display());
            return Some(config_path);
        };
        warn!("$XDG_CONFIG_HOME environment variable not set or not valid unicode. using $HOME/.config instead.");
        if let Ok (home_dir) = var(home_key) {
            trace!("found $HOME: {home_dir}");
            let mut config_path = PathBuf::from(home_dir);
            config_path.push(".config/brr/brr.conf");
            info!("using config path: {}", config_path.display());
            return Some(config_path);
        };
        warn!("$HOME environment variable not set or not valid unicode. checking executable path instead.");
    } else if OS == "linux" {
        if let Ok(state_dir) = var(state_key) {
            trace!("found $XDG_STATE_HOME: {state_dir}");
            let mut log_path = PathBuf::from(state_dir);
            log_path.push(PathBuf::from("brr"));
            
            match create_dir_all(&log_path) {
                Ok(()) => {
                    log_path.push("brr.log");
                    info!("using log path: {}", log_path.display());
                    return Some(log_path);
                },
                Err(error_msg) => {
                    error!("[metadata.rs]: {error_msg} - could not create log directory {}. using executable path instead.", log_path.display());
                },
            };
        } else {
            warn!("$XDG_STATE_HOME environment variable not set or not valid unicode. using $HOME/.local/state instead.");
        };
        if let Ok (home_dir) = var(home_key) {
            trace!("found $HOME: {home_dir}");
            let mut log_path = PathBuf::from(home_dir);
            log_path.push(".local/state/brr");

            match create_dir_all(&log_path) {
                Ok(()) => {
                    log_path.push("brr.log");
                    info!("using log path: {}", log_path.display());
                    return Some(log_path);
                },
                Err(error_msg) => {
                    error!("[metadata.rs]: {error_msg} - could not create log directory {}. using executable path instead.", log_path.display());
                },
            };
        } else {
            warn!("$HOME environment variable not set or not valid unicode. using executable path instead.");
        };
    }
    current_exe_path(config)
}

#[allow(clippy::needless_return)] // seems to be a false positive
fn current_exe_path(config: bool) -> Option<PathBuf> {
    match current_exe() {
        Ok(exe_path) => {
            if let Some(parent) = exe_path.parent() {
                let mut path = parent.to_path_buf();
                
                if config {
                    path.push(PathBuf::from("brr.conf"));

                    info!("using config path: {}", path.display());
                    return Some(path);
                };
                path.push(PathBuf::from("brr.log"));
            
                return Some(path);
            };
            error!("[metadata.rs]: executable seems to have no parent folder. using default config.");
            return None;
        },
        Err(error_msg) => {
            error!("[metadata.rs]: {error_msg} - could not find current executable path. using default config.");
            return None;
        },
    };
}
