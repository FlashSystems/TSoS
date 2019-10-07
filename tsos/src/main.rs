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
use config::Id;

#[derive(Debug)]
pub enum Error {
	ProviderNotFound(String),
	ProviderFailed(PathBuf, i32),
	ProviderTerminated(PathBuf),
	ProviderNoFile(PathBuf),
	TemplateNotFound(String, String),
	InvalidSourceName(String),
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::ProviderNotFound(sos) => write!(f, "Provider {} not found in search path.", sos),
			Self::ProviderFailed(provider_file, result_code) => write!(f, "Provider {} failed to execute with result code {}.", provider_file.display(), result_code),
			Self::ProviderTerminated(provider_file) => write!(f, "Provider {} terminated by signal.", provider_file.display()),
			Self::ProviderNoFile(provider_file) => write!(f, "{} is not a file.", provider_file.display()),
			Self::TemplateNotFound(sos, source_file) => write!(f, "Template file {} for secret provider {} not found.", source_file, sos),
			Self::InvalidSourceName(sos) => write!(f, "Invalid source name {}.", sos)
		}
	}
}

impl error::Error for Error {
	fn source(&self) -> Option<&(dyn error::Error + 'static)> {
		None
	}
}

/// Variant of start_logger that tries to detect an active jourald an
/// switches to journal logging if it is enabled.
#[cfg(feature = "systemd")]
fn start_logger(log_level: Level) {
	let force = if let Ok(force_env) = env::var("TSOS_FORCE_JOURNAL") {
		if let Some(first_char) = force_env.chars().next() {
			[ '1', 'y', 'Y', 't', 'T' ].iter().find(|&&v| v == first_char).is_some()
		} else {
			false
		}
	} else {
		false
	};

	if force || journal_logger::has_journal() {
		journal_logger::init_with_level(log_level).unwrap();
		debug!("Journal logging detected. Switch to journal logger completed.");
	} else {
		simple_logger::init_with_level(log_level).unwrap();
		debug!("No journal logging detected. Using default stderr logger.");
	}
}

/// Variant of start_logger that always uses simple_logger if the systemd feature
/// is disabled.
#[cfg(not(feature = "systemd"))]
fn start_logger(log_level: Level) {
	simple_logger::init_with_level(log_level).unwrap();
}

fn find_provider(search_path: &[PathBuf], provider_name: &OsStr) -> Option<PathBuf> {
	for path in search_path {
		if path.is_dir() {
			let mut provider_path = PathBuf::from(path);
			provider_path.push(provider_name);

			debug!("Trying {} as secret provider...", provider_path.display());

			//TODO: Check the file is only writable by root.

			if provider_path.is_file() {
				return Some(provider_path);
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

	for (sos, templates) in config.local.secrets.iter() {
		debug!("Processing secret provider {}...", sos);

		// Make sure the file name can not be used for path traversal attacks
		let sos = Path::new(sos).file_name().ok_or_else(|| Error::InvalidSourceName(sos.clone()))?;

		// Search for the secret provider
		// If a local search path is configured it takes precedence over the global search path.
		let mut provider_search_result = if let Some(ref search_path) = config.local.search_path { find_provider(search_path, sos) } else { None };
		if provider_search_result.is_none() {
			provider_search_result = find_provider(&config.global.search_path, sos);
		}

		if let Some(provider_file) = provider_search_result {
			debug!("Found secret provider {} for secret {}.", provider_file.display(), sos.to_string_lossy());
			for template in templates.iter() {
				let target = temp.create_file("tsos-final")?;
				let template = Path::new(template);	// Shadow target with a path instance because we need a path more often than a string.

				if template.is_file() {
					debug!("Executing secret provider...");

					// Execute the secret provider.
					// It will use the input file ($1) and update the output file ($2).
					let exit_code = Command::new(&provider_file).args(&[template, &target]).status()?;
					if !exit_code.success() {
						if let Some(code) = exit_code.code() {
							return Err(Box::new(Error::ProviderFailed(provider_file, code)));
						} else {
							return Err(Box::new(Error::ProviderTerminated(provider_file)));
						}
					}

					debug!("Copying permissions...");

					system::copy_perms_and_owners(&template, &target)?;

					system::bind(&target, &template)?;
				} else {
					return Err(Box::new(Error::TemplateNotFound(sos.to_string_lossy().into_owned(), template.to_string_lossy().into_owned())));
				}
			}
		} else {
			return Err(Box::new(Error::ProviderNotFound(sos.to_string_lossy().into_owned())));
		}
	}

	Ok(())
}

fn prepare_privileges(command: &mut Command, config: &Config) -> Result<(), Box<dyn error::Error>> {
	let uid_gid = match config.local.uid {
		Some(Id::Nummeric(ref uid)) => Some(system::resolve_uid(*uid)?),
		Some(Id::Text(ref user_name)) => Some(system::resolve_user(user_name)?),
		None => None
	};
	let gid = match config.local.gid {
		Some(Id::Nummeric(ref gid)) => Some(*gid),
		Some(Id::Text(ref group_name)) => Some(system::resolve_group(group_name)?),
		None => None
	};

	if let Some((uid, ugid)) = uid_gid { command.uid(uid); command.gid(ugid); }
	if let Some(gid) = gid { command.gid(gid); }
	
	Ok(())
}

/// WARNING: This function ends in an execvp. No destructors for instances allocated
/// within this function will run. All preparation is done in the prepare() function.
/// When this function terminates all destructors (drop) will run and everything is
/// fine. Therefore do all RAII within prepare!
/// This method only allocates a Logger- and a Config-Instance. These will not been
/// torn down. They simply will vanish when the process memory is replaced with the
/// new process image.
fn main() {
	// Parse the TSOS_LOG environment variable and set the log-level accoringly.
	let log_level = match env::var("TSOS_LOG") {
		Ok(level) => Level::from_str(&level).unwrap_or(Level::Warn),
		Err(_) => Level::Warn
	};

	start_logger(log_level);

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

			let mut command = Command::new(&config.local.exec);
			if let Err(error) = prepare_privileges(&mut command, &config) {
				error!("Preparing privileges for executing {} failed: {}", config.local.exec.display(), error);
				exit(4);
			}
			command.args(args);
			command.exec();
		},
		Err(error) => {
			error!("Failed to parse configuration file {} ", error);
			exit(2);
		}
	}
}
