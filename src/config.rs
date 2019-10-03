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
#[serde(untagged)]
pub enum Id {
	Nummeric(u32),
	Text(String)
}

#[derive(Debug, Deserialize)]
pub struct Local {
	pub exec: PathBuf,
	pub env_path: Option<bool>,
	pub secrets: HashMap<String, Vec<String>>,
	pub search_path: Option<Vec<PathBuf>>,
	pub uid: Option<Id>,
	pub gid: Option<Id>
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

/// WARNING: This class must be prepared to vanish at any moment without getting
/// its destructor called. Do not use resources thar require RAII!
impl Config {
	pub fn new(file: &Path, env_path: Option<OsString>) -> Result<Self, Error> {
		// Read and parse the configuration file
		let mut config_string = String::new();
		File::open(file)?.read_to_string(&mut config_string)?;
		let local_config: Local = toml::from_str(&config_string)?;

		// Create a list of search paths used for searching for secret provider scripts
		let mut search_path = Vec::with_capacity(2);

		// If the usage of an envrionment variable for the search path is enabled and
		// an additional path was supplied, append it to the default search path.
		if local_config.env_path.unwrap_or(false) {
			if let Some(env_path) = env_path {
				for path in split_paths(&env_path) {
					search_path.push(path);
				}
			}
		}

		// Append the default paths as a last resort.
		search_path.push(PathBuf::from("/etc/tsos.d"));
		search_path.push(PathBuf::from("/usr/lib/tsos"));

		Ok(Self{
			local: local_config,
			global: Global {
				search_path
			}
		})
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	#[should_panic(expected="missing field `exec`")]
	fn missing_exec() {
		let toml = r#"
			uid = 10
			gid = "test"

			[secrets]
		"#;
		let _parsed: Local = toml::from_str(toml).unwrap();
	}

	#[test]
	#[should_panic(expected="missing field `secrets`")]
	fn missing_secrets() {
		let toml = r#"
			exec = "test"
			uid = 10
			gid = "test"
		"#;
		let _parsed: Local = toml::from_str(toml).unwrap();
	}

	#[test]
	fn nummeric_uid_gid() {
		let toml = r#"
			exec = "test"
			uid = 10
			gid = 20

			[secrets]
		"#;
		let parsed: Local = toml::from_str(toml).unwrap();

		match parsed.uid.unwrap() {
			Id::Nummeric(uid) => assert_eq!(uid, 10),
			_ => assert!(false, "UID is not of variant nummeric.")
		}
		match parsed.gid.unwrap() {
			Id::Nummeric(gid) => assert_eq!(gid, 20),
			_ => assert!(false, "GID is not of variant nummeric.")
		}
	}

	#[test]
	fn check_parser() {
		let toml = r#"
			exec = "test"
			uid = "user"
			gid = "group"
			search_path = [ "/a", "/b" ]

			[secrets]
				first = [ "/fa", "/fb" ]
				second = [ "/sa", "/sb" ]
		"#;
		let parsed: Local = toml::from_str(toml).unwrap();

		// Check UID
		match parsed.uid.unwrap() {
			Id::Text(user) => assert_eq!(user, "user"),
			_ => assert!(false, "UID is not text.")
		}
		match parsed.gid.unwrap() {
			Id::Text(group) => assert_eq!(group, "group"),
			_ => assert!(false, "GID is not text.")
		}

		// Check exec
		assert_eq!(parsed.exec.to_string_lossy(), "test");

		// Check search path
		let mut search_path = parsed.search_path.unwrap();
		assert_eq!(search_path.remove(0).to_string_lossy(), "/a");
		assert_eq!(search_path.remove(0).to_string_lossy(), "/b");
	}
}