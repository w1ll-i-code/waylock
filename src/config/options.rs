use std::ops::Add;
use std::env;
use std::path::PathBuf;

use serde::Deserialize;
use structopt::StructOpt;

use crate::config::{color, ConfigError};
use log::{error, LevelFilter};

#[derive(Debug, StructOpt, Deserialize)]
pub struct Options {
    #[structopt(long, verbatim_doc_comment)]
    /// Command to be executed on a wrong entry of a password
    pub fail_command: Option<String>,

    #[structopt(short = "v", parse(from_occurrences), verbatim_doc_comment)]
    #[serde(skip)]
    /// Log level, [None, Error, Warning, Info, Debug, Trace]. Default None.
    pub log_level: u64,

    #[structopt(short, long, parse(from_os_str), verbatim_doc_comment)]
    #[serde(skip)]
    /// Path to a config file. Default ~/.config/waylock/
    pub config: Option<PathBuf>,

    #[structopt(long)]
    /// Font for the GUI
    pub font: Option<String>,

    #[structopt(long, verbatim_doc_comment)]
    /// Max restarts/seconds before it stops to restart.
    pub max_restarts: Option<usize>,

    #[structopt(flatten, verbatim_doc_comment)]
    #[serde(default)]
    pub colors: Colors,
}

#[derive(Debug, StructOpt, Deserialize, Default)]
pub struct Colors {
    #[structopt(short = "C", long, parse(try_from_str = color::from_str), verbatim_doc_comment)]
    /// Color of the GUI bar, when the lock is initialized
    pub color_init: Option<u32>,

    #[structopt(short = "C", long, parse(try_from_str = color::from_str), verbatim_doc_comment)]
    /// Color of the GUI bar, during typing
    pub color_input: Option<u32>,

    #[structopt(long, parse(try_from_str = color::from_str), verbatim_doc_comment)]
    /// Color of the GUI bar, if the password was wrong
    pub color_fail: Option<u32>,

    #[structopt(long, parse(try_from_str = color::from_str), verbatim_doc_comment)]
    /// Static background color of the GUI.
    pub color_bg: Option<u32>,

    #[structopt(long, parse(try_from_str = color::from_str), verbatim_doc_comment)]
    /// Color of the text displayed
    pub color_text: Option<u32>,
}

fn default_config_path() -> Result<PathBuf, ConfigError> {
    let home = |_| env::var("HOME").map(|v| v.add("/.config"));
    env::var("XDG_CONFIG_HOME")
        .or_else(home)
        .map(|path| path.add("/waylock/waylock.toml"))
        .map(PathBuf::from)
        .map_err(ConfigError::Env)
}



impl Options {
    pub fn new() -> Result<Options, ConfigError> {
        let mut cmd_params: Options = Options::from_args_safe().map_err(ConfigError::Params)?;

        let _ = crate::logger::Logger::init(match cmd_params.log_level {
            0 => LevelFilter::Off,
            1 => LevelFilter::Error,
            2 => LevelFilter::Warn,
            3 => LevelFilter::Info,
            4 => LevelFilter::Debug,
            _ => LevelFilter::Trace,
        });

        let config_path = match cmd_params.config.clone() {
            None => default_config_path()?,
            Some(config) => config
        };

        if config_path.exists() {
            let file = std::fs::read_to_string(config_path).map_err(ConfigError::IO)?;
            let config_file_options = toml::from_str(&file).map_err(ConfigError::Toml)?;
            cmd_params = cmd_params.or(config_file_options);
        } else {
            error!("Could not load a configuration file");
        }

        Ok(cmd_params)
    }

    fn or(mut self, other: Self) -> Self {
        if self.fail_command.is_none() { self.fail_command = other.fail_command; }
        if self.font.is_none() { self.font = other.font; }

        if self.colors.color_init.is_none() { self.colors.color_init = other.colors.color_init; }
        if self.colors.color_input.is_none() { self.colors.color_input = other.colors.color_input; }
        if self.colors.color_fail.is_none() { self.colors.color_fail = other.colors.color_fail; }

        if self.colors.color_bg.is_none() { self.colors.color_bg = other.colors.color_bg; }
        if self.colors.color_text.is_none() { self.colors.color_text = other.colors.color_text; }

        self
    }
}

