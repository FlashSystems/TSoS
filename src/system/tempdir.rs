use libc;
use std::path::{Path, PathBuf};
use std::ffi::{CString};
use std::env::temp_dir;
use std::fs::remove_dir_all;
use std::io;

use super::Error;

pub struct TempDir {
	path: PathBuf
}

impl Drop for TempDir {
	fn drop(&mut self) {
		// We ignore the result here because there is not much we can do
		// if deleting the temporary path fails.
		let _ = remove_dir_all(&self.path);
	}
}

// Implement the From-Trait for TempDir so it can be used everywhere
//  a Path can be used.
impl<'t> From<&'t TempDir> for &'t Path {
	fn from(temp_dir: &'t TempDir) -> Self {
		&temp_dir.path
	}
}
impl AsRef<Path> for TempDir {
	fn as_ref(&self) -> &Path {
		&self.path
	}
}

impl TempDir {
	pub fn new(prefix: &str) -> Result<Self, Error> {
		// Create a template for the temporary directory.
		let mut temp_dir = temp_dir();
		temp_dir.push(format!("{}-XXXXXX", prefix));

		let c_temp_dir = CString::new(temp_dir.to_str().unwrap())?;

		// Reserve the temporary directory and get the name returned by
		// mkdtemp
		let c_temp_dir = unsafe { libc::mkdtemp(c_temp_dir.into_raw()) };

		//FIXME: c_temp_dir is not freed
		if c_temp_dir.is_null() {
			Err(Error::from(io::Error::last_os_error()))
		} else {
			Ok(Self {
				path: unsafe { PathBuf::from(CString::from_raw(c_temp_dir).into_string()?) }
			})
		}
	}

	pub fn create_file() -> Result<PathBuf, Error> {

	}
}
