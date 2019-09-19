use std::path::{Path, PathBuf};
use std::error;
use std::process::Command;
use std::fmt;
use std::ffi::OsStr;
use std::os::linux::fs::MetadataExt;

use simple_logger;
use log::{Level, debug, error};

mod system;
use system::TempDir;

mod config;
use config::Config;

#[derive(Debug)]
pub enum Error {
	ProviderNotFound(PathBuf),
	ProviderFailed(PathBuf, i32),
	ProviderTerminated(PathBuf),
	ProviderNoFile(PathBuf),
	TemplateNotFound(String, String),
	InvalidSourceName(String),
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::ProviderNotFound(script_file) => write!(f, "Provider {} not found in search path.", script_file.display()),
			Self::ProviderFailed(script_file, result_code) => write!(f, "Provider {} failed to execute with result code {}.", script_file.display(), result_code),
			Self::ProviderTerminated(script_file) => write!(f, "Provider {} terminated by signal.", script_file.display()),
			Self::ProviderNoFile(script_file) => write!(f, "{} is not a file.", script_file.display()),
			Self::TemplateNotFound(sos, source_file) => write!(f, "Source file {} for secret provider {} not found.", source_file, sos),
			Self::InvalidSourceName(sos) => write!(f, "Invalid source name {}.", sos)
		}
	}
}

impl error::Error for Error {
	fn source(&self) -> Option<&(dyn error::Error + 'static)> {
		None
	}
}

fn check_permissions(metadata: &MetadataEx) {
	
}

fn find_script(search_path: &Vec<PathBuf>, script_name: &OsStr) -> Option<PathBuf> {
	for path in search_path {
		if path.is_dir() {
			let mut script_path = PathBuf::from(path);
			script_path.push(script_name);

			if let Ok(meta) = script_path.metadata() {
				if meta.st_uid() == 0 && meta.st_gid() == 0 && meta.st_mode() & {

				}
			}

			if script_path.is_file() {
				return Some(script_path);
			}
		}
	}

	None
}

fn go(config: &Config) -> Result<(), Box<dyn error::Error>> {
	//system::unshare_mount_ns()?;

	// Create temporary directory and mount a ramfs onto it
	let mut temp = TempDir::new("tsos")?;
	//system::mount_ramfs(512, "tsos", temp.as_ref())?;

	for (sos, targets) in config.local.secrets.iter() {
		// Make sure the file name can not be used for path traversal attacks
		let sos = Path::new(sos).file_name().ok_or_else(|| Error::InvalidSourceName(sos.clone()))?;

		if let Some(script_file) = find_script(&config.global.search_path, sos) {
			for target in targets.iter() {
				let destination = temp.create_file("tsos-dst")?;

				if Path::new(target).is_file() {
					debug!("Using secret provider {} for {}.", script_file.display(), Path::new(sos).display());

					let exit_code = Command::new(&script_file).args(&[target, destination.to_str().unwrap_or("")]).status()?;
					if !exit_code.success() {
						if let Some(code) = exit_code.code() {
							return Err(Box::new(Error::ProviderFailed(script_file, code)));
						} else {
							return Err(Box::new(Error::ProviderTerminated(script_file)));
						}
					}
				} else {
					return Err(Box::new(Error::ProviderNoFile(script_file)));
				}
			}
		} else {
			return Err(Box::new(Error::ProviderNotFound(PathBuf::from(sos))));
		}
	}

	//system::spawn_wait(&PathBuf::from("/usr/bin/ls"), &vec!("-la", "/tmp")).expect("ls");
	//system::spawn_wait(&PathBuf::from("/usr/bin/ls"), &vec!("-la", temp.as_ref().to_str().unwrap())).expect("ls");

	//system::umount(temp.as_ref());

	Ok(())
}

fn main() {
	simple_logger::init_with_level(Level::Debug).unwrap();

	let config = Config::new(&PathBuf::from("./test/myprog.toml"));

	go(&config.unwrap()).unwrap();


	//system::spawn_wait(&PathBuf::from("/usr/bin/sleep"), &vec!("10")).expect("spawn_wait");
	//system::spawn_wait(&PathBuf::from("/usr/bin/echo"), &vec!("-e", "asd")).expect("spawn_wait");


	print!("DONE"); 
}
