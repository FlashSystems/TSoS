use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::{self, Read};
use std::collections::HashMap;
use std::error;
use std::fmt;

#[derive(Debug)]
pub enum Error {
	IoError(io::Error),
	ParseError(toml::de::Error)
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::IoError(error) => write!(f, "I/O error: {}", error),
			Self::ParseError(error) => write!(f, "Prase error: {}", error)
		}
	}
}

impl error::Error for Error {
	fn source(&self) -> Option<&(dyn error::Error + 'static)> {
		match self {
			Self::IoError(error) => Some(error),
			Self::ParseError(error) => Some(error)
		}
	}
}

impl From<io::Error> for Error {
	fn from(error: io::Error) -> Self {
		Self::IoError(error)
	}
}

impl From<toml::de::Error> for Error {
	fn from(error: toml::de::Error) -> Self{
		Self::ParseError(error)
	}
}

#[derive(Debug, Deserialize)]
pub struct Config {
	exec: PathBuf,
	secrets: HashMap<String, Vec<String>>
}

impl Config {
	pub fn new(file: &Path) -> Result<Self, Error> {
		let mut config_file = File::open(file)?;
		let mut config_string = String::new();

		config_file.read_to_string(&mut config_string)?;

		Ok(toml::from_str(&config_string)?)
	}
}