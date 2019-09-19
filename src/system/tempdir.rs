use libc;
use log::{debug, warn};
use std::path::{Path, PathBuf};
use std::ffi::{CString};
use std::env::temp_dir;
use std::fs::remove_dir_all;
use std::io;

use super::Error;

pub struct TempDir {
	path: PathBuf,
	next_id: u32
}

impl Drop for TempDir {
	fn drop(&mut self) {
		debug!("Deleting temporary directory {}...", self.path.display());

		// We ignore the result here because there is not much we can do
		// if deleting the temporary path fails.
		if let Err(error) = remove_dir_all(&self.path) {
			warn!("Deleting temporary directory {} failed: {}", self.path.display(), error);
		}
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
			let temp_dir = unsafe { PathBuf::from(CString::from_raw(c_temp_dir).into_string()?) };

			debug!("Allocated temporary directory {}.", temp_dir.display());

			Ok(Self {
				path: temp_dir,
				next_id: 0
			})
		}
	}

	pub fn create_file(&mut self, prefix: &str) -> Result<PathBuf, Error> {
		let mut temp_file = self.path.clone();

		temp_file.push(format!("{}-{:08x}", prefix, self.next_id));
		self.next_id+=1;

		// We unwrap here because we know that all parts are valid because they are path components.
		let c_temp_file = CString::new(temp_file.to_str().unwrap())?;

		let h_file = unsafe { libc::open(c_temp_file.as_ptr(), libc::O_CREAT|libc::O_NOFOLLOW|libc::O_TRUNC|libc::O_WRONLY, libc::S_IRWXU) };

		if h_file < 0 {
			Err(Error::from(io::Error::last_os_error()))
		} else {
			debug!("Allocated temporary file {}.", temp_file.display());

			// Close the temporary file and return its name
			unsafe { libc::close(h_file) };
			Ok(temp_file)
		}
	}
}
