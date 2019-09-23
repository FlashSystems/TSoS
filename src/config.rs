use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::{self, Read};
use std::ffi::OsString;
use std::collections::HashMap;
use std::error;
use std::fmt;
use std::env::split_paths;

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
pub struct Local {
	pub exec: PathBuf,
	pub secrets: HashMap<String, Vec<String>>,
	pub search_path: Option<Vec<PathBuf>>
}

#[derive(Debug)]
pub struct Global {
	pub search_path: Vec<PathBuf>
}

#[derive(Debug)]
pub struct Config {
	pub local: Local,
	pub global: Global
}

impl Config {
	pub fn new(file: &Path, env_path: Option<OsString>) -> Result<Self, Error> {
		let mut config_file = File::open(file)?;
		let mut config_string = String::new();

		config_file.read_to_string(&mut config_string)?;

		// Create a list of search paths used for searching for secret provider scripts
		let mut search_path = Vec::with_capacity(2);

		// If an additional path was supplied, append it to the default search path.
		if let Some(env_path) = env_path {
			for path in split_paths(&env_path) {
				search_path.push(path);
			}
		};

		// Append the default paths as a last resort.
		search_path.push(PathBuf::from("/etc/tsos.d"));
		search_path.push(PathBuf::from("/usr/lib/tsos"));

		Ok(Self{
			local: toml::from_str(&config_string)?,
			global: Global {
				search_path
			}
		})
	}
}

#[cfg(test)]
mod test {
}