use std::path::{Path, PathBuf};
use std::error;
use std::process::Command;
use std::env;
use std::os::unix::process::CommandExt;
use std::fmt;
use std::str::FromStr;
use std::ffi::OsStr;
use std::process::exit;

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

fn find_script(search_path: &[PathBuf], script_name: &OsStr) -> Option<PathBuf> {
	for path in search_path {
		if path.is_dir() {
			let mut script_path = PathBuf::from(path);
			script_path.push(script_name);

			debug!("Trying {} as secret provider...", script_path.display());

			//TODO: Check the file is only writable by root.

			if script_path.is_file() {
				return Some(script_path);
			}
		} else {
			debug!("Search path {} not found or no directory.", path.display());
		}
	}

	None
}

fn prepare(config: &Config) -> Result<(), Box<dyn error::Error>> {
	system::unshare_mount_ns()?;

	// Create temporary directory and mount a ramfs onto it
	let mut temp = TempDir::new("tsos")?;
	
	let _temp_mount = system::RamFs::new("tsos", temp.as_ref())?;

	for (sos, targets) in config.local.secrets.iter() {
		debug!("Processing secret provider {}...", sos);

		// Make sure the file name can not be used for path traversal attacks
		let sos = Path::new(sos).file_name().ok_or_else(|| Error::InvalidSourceName(sos.clone()))?;

		// Search for the secret provider script
		// If a local search path is configured it takes precedence over the global search path.
		let mut script_search_result = if let Some(ref search_path) = config.local.search_path { find_script(search_path, sos) } else { None };
		if script_search_result.is_none() {
			script_search_result = find_script(&config.global.search_path, sos);
		}

		if let Some(script_file) = script_search_result {
			debug!("Found secret provider {} for secret {}.", script_file.display(), Path::new(sos).display());
			for target in targets.iter() {
				let destination = temp.create_file("tsos-dst")?;
				let target = Path::new(target);	// Shadow target with a path instance because we need a path more often than a string.

				if target.is_file() {
					debug!("Executing secret provider...");

					// Execute the secret provider.
					// It will use the input file ($1) and update the output file ($2).
					let exit_code = Command::new(&script_file).args(&[target, &destination]).status()?;
					if !exit_code.success() {
						if let Some(code) = exit_code.code() {
							return Err(Box::new(Error::ProviderFailed(script_file, code)));
						} else {
							return Err(Box::new(Error::ProviderTerminated(script_file)));
						}
					}

					debug!("Copying permissions...");

					system::copy_perms_and_owners(&target, &destination)?;

					system::bind(&destination, &target)?;
				} else {
					return Err(Box::new(Error::ProviderNoFile(script_file)));
				}
			}
		} else {
			return Err(Box::new(Error::ProviderNotFound(PathBuf::from(sos))));
		}
	}

	Ok(())
}

fn main() {
	// Parse the TSOS_LOG environment variable and set the log-level accoringly.
	let log_level = match env::var("TSOS_LOG") {
		Ok(level) => Level::from_str(&level).unwrap_or(Level::Warn),
		Err(_) => Level::Warn
	};

	simple_logger::init_with_level(log_level).unwrap();

	// Extract the command line arguments and check if we got at least one
	// argument (the config file name). All other arguments will be passed
	// down to the final program hat we execute.
	let mut args: Vec<String> = env::args().collect();
	args.remove(0); // Remove the first argument as it is our name.

	if args.is_empty() {
		error!("Missing configuration file command line parameter.");
		exit(1);
	}

	match Config::new(&PathBuf::from(args.remove(0)), std::env::var_os("TSOS_PATH")) {
		Ok(config) => {
			if let Err(error) = prepare(&config) {
				error!("Starting {} with TSOS failed: {}", config.local.exec.display(), error);
				exit(3);
			}

			debug!("Replacing this process with {}...", config.local.exec.display());

			Command::new(config.local.exec).args(args).exec();
		},
		Err(error) => {
			error!("Failed to parse configuration file {} ", error);
			exit(2);
		}
	}
}
