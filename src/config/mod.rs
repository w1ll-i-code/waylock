use std::fmt::{Display, Formatter};
use std::string::ToString;

use fontdue::Font;
use serde::de::{Error, StdError};
use crate::config::options::Options;
use crate::config::font::load_font;
use std::io::ErrorKind;
use std::env::VarError;
use crate::config::color::Error as ColorError;

mod color;
mod options;
mod font;

#[derive(Debug)]
pub enum ConfigError {
    IO(std::io::Error),
    Toml(toml::de::Error),
    Color(color::Error),
    Params(clap::Error),
    Env(std::env::VarError),
    Serde(String)
}

impl Display for ConfigError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::IO(err) => match err.kind() {
                ErrorKind::NotFound => f.write_str("Could not find the specified config file."),
                ErrorKind::PermissionDenied => f.write_str("Could not access the specified config file."),
                ErrorKind::InvalidData | ErrorKind::InvalidInput => f.write_str("Configuration file seems to contain invalid utf-8 data."),
                _ => f.write_str("An unknown issue occurred while trying to read the config file.")
            }
            ConfigError::Toml(err) => match err.line_col() {
                None => f.write_str("Unknown error occurred while reading the configuration."),
                Some((line, col)) => f.write_str(&format!("Error occurred while parsing the config file at line {}, column {}.", line + 1, col + 1)),
            }
            ConfigError::Color(err) => match err {
                ColorError::InvalidLength => f.write_str("Error while parsing the flags. A color seems to have invalid length"),
                ColorError::InvalidPrefix => f.write_str("Error while parsing the flags. A color seems to have an invalid prefix"),
                ColorError::ParseInt(_) => f.write_str("Error while parsing the flags. A color seems to be ill formated."),
            },
            ConfigError::Params(err) => f.write_str(&err.message),
            ConfigError::Env(err) => match err {
                VarError::NotPresent => f.write_str("Both $XDG_CONFIG_HOME and $HOME don't seem to be present."),
                VarError::NotUnicode(_) => f.write_str("Either $XDG_CONFIG_HOME or $HOME are not in Unicode.")
            },
            ConfigError::Serde(err) => f.write_str(&format!("Could not parse the config file. Error: {}", err)),
        }
    }
}

impl StdError for ConfigError {}

impl Error for ConfigError {
    fn custom<T>(msg: T) -> Self where T: Display {
        ConfigError::Serde(msg.to_string())
    }
}
pub struct Config {
    pub fail_command: Option<String>,
    pub font: [Font; 1],
    pub user: String,
    pub colors: Colors,
}

pub struct Colors {
    pub init_color: u32,
    pub input_color: u32,
    pub fail_color: u32,
    pub bg_color: u32,
    pub text_color: u32,
}

impl Config {
    pub fn new() -> Result<Config, ConfigError> {
        Ok(Options::new()?.into())
    }
}

impl From<Options> for Config {
    fn from(options: Options) -> Self {
        let font = options.font.map(load_font)
            .flatten()
            .or_else(|| load_font("monospace"))
            .expect("The default font is not available on the system.");

        let user = users::get_current_username().expect("No user is running this command");

        Self {
            fail_command: options.fail_command,
            font: [font],
            user: user.into_string().expect("Username could not be fetched"),
            colors: Colors {
                init_color: options.colors.init_color.unwrap_or(0xffffffff) | 0xff000000,
                input_color: options.colors.input_color.unwrap_or(0xff0000ff) | 0xff000000,
                fail_color: options.colors.fail_color.unwrap_or(0xffff0000) | 0xff000000,
                bg_color: options.colors.bg_color.unwrap_or(0xff000000) | 0xff000000,
                text_color: options.colors.text_color.unwrap_or(0xffffffff) | 0xff000000,
            }
        }
    }
}

#[test]
fn test() {
    let font = font_loader::system_fonts::get(
        &font_loader::system_fonts::FontPropertyBuilder::new().family("monospace").build()
    );

    assert!(font.is_some())
}