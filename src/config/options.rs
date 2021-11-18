use std::ops::Add;
use std::env;
use std::path::PathBuf;

use serde::Deserialize;
use structopt::StructOpt;

use crate::config::{color, ConfigError};
use log::{error, LevelFilter};

#[derive(Debug, StructOpt, Deserialize)]
pub struct Options {
    #[structopt(long)]
    pub fail_command: Option<String>,

    #[structopt(short = "v", parse(from_occurrences))]
    #[serde(skip)]
    pub log_level: u64,

    #[structopt(long, parse(from_os_str))]
    #[serde(skip)]
    pub config: Option<PathBuf>,

    #[structopt(long)]
    pub font: Option<String>,

    #[structopt(long)]
    pub max_restarts: Option<usize>,

    #[structopt(flatten)]
    #[serde(default)]
    pub colors: Colors,
}

#[derive(Debug, StructOpt, Deserialize, Default)]
pub struct Colors {
    #[structopt(long, parse(try_from_str = color::from_str))]
    pub init_color: Option<u32>,

    #[structopt(long, parse(try_from_str = color::from_str))]
    pub input_color: Option<u32>,

    #[structopt(long, parse(try_from_str = color::from_str))]
    pub fail_color: Option<u32>,

    #[structopt(long, parse(try_from_str = color::from_str))]
    pub bg_color: Option<u32>,

    #[structopt(long, parse(try_from_str = color::from_str))]
    pub text_color: Option<u32>,
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

        if self.colors.init_color.is_none() { self.colors.init_color = other.colors.init_color; }
        if self.colors.input_color.is_none() { self.colors.input_color = other.colors.input_color; }
        if self.colors.fail_color.is_none() { self.colors.fail_color = other.colors.fail_color; }

        if self.colors.bg_color.is_none() { self.colors.bg_color = other.colors.bg_color; }
        if self.colors.text_color.is_none() { self.colors.text_color = other.colors.text_color; }

        self
    }
}

