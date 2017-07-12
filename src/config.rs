///Config module: used for loading the bot's config from an external file.
///This can be useful for running the bot permanently, and having differences between instances -
///i.e. unique tokens and command prefixes.

use std::io;
use std::io::Read;
use std::fs::File;
use std::error;
use std::fmt;
use std::path::Path;

use toml;

use self::defaults::*;

mod defaults {
    pub fn default_prefix() -> Vec<String> {
        vec!["+".to_owned()]
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    ///The bot's login token.
    pub token: String,
    ///The bot's prefix. Default is '+'
    #[serde(default = "default_prefix")]
    pub prefixes: Vec<String>,
}
impl Config {
    pub fn load_from<P: AsRef<Path>>(path: P) -> Result<Config, ConfigError> {
        //load the config file into a String
        let mut config_file = File::open(path)?;
        let mut config = String::new();
        config_file.read_to_string(&mut config)?;

        let config: Config = toml::from_str(config.as_str())?; //parse/deserialize the config file
        if config.prefixes.len() == 0 {
            return Err(ConfigError::Invalid(
                "At least one prefix is required".to_owned(),
            ));
        }
        Ok(config)
    }
    pub fn new() -> Config {
        Config {
            token: "".to_owned(),
            prefixes: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Io(io::Error),
    Deserialize(toml::de::Error),
    Invalid(String),
}
impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let message = match *self {
            ConfigError::Io(ref e) => e.to_string(),
            ConfigError::Deserialize(ref e) => e.to_string(),
            ConfigError::Invalid(ref message) => message.clone(),
        };
        write!(f, "{}", message)
    }
}
impl error::Error for ConfigError {
    fn description(&self) -> &str {
        match *self {
            ConfigError::Io(ref e) => e.description(),
            ConfigError::Deserialize(ref e) => e.description(),
            ConfigError::Invalid(_) => {
                "The config was successfully loaded, but contained invalid data."
            }
        }
    }
}
impl From<io::Error> for ConfigError {
    fn from(e: io::Error) -> ConfigError {
        ConfigError::Io(e)
    }
}
impl From<toml::de::Error> for ConfigError {
    fn from(e: toml::de::Error) -> ConfigError {
        ConfigError::Deserialize(e)
    }
}
