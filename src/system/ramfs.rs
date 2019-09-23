use libc;
use log::{debug, warn};
use std::path::{PathBuf, Path};
use std::ffi::CString;
use std::ptr;
use std::io;

pub struct RamFs {
	mount_point: PathBuf
}

impl Drop for RamFs {
	fn drop(&mut self) {
		//FIXME: Error handling?
		let c_path = CString::new(self.mount_point.to_str().unwrap()).unwrap();

		debug!("Unmounting {}...", self.mount_point.display());

		if unsafe { libc::umount(c_path.as_ptr()) } < 0 {
			warn!("Unmounting {} failed with error {}", self.mount_point.display(), io::Error::last_os_error());
		}
	}
}

impl RamFs {
	pub fn new(source_tag: &str, path: &Path) -> io::Result<Self> {
		let c_source_tag = CString::new(source_tag)?;
		let c_path = CString::new(path.to_str().unwrap())?;
		let c_fstype = CString::new("ramfs")?;
		let c_params = CString::new("mode=701")?;

		debug!("Mounting ramfs on {}...", path.display());

		if unsafe { libc::mount(c_source_tag.as_ptr(), c_path.as_ptr(), c_fstype.as_ptr(), libc::MS_NODEV|libc::MS_NOEXEC, c_params.as_ptr() as *const libc::c_void) } < 0 {
			return Err(io::Error::last_os_error());
		}

		debug!("Making mount {} private...", path.display());

		// Make the mount private. We don't want this mount point to propagate anywhere.
		if unsafe { libc::mount(ptr::null(), c_path.as_ptr(), ptr::null(), libc::MS_PRIVATE, ptr::null()) } < 0 {
			Err(io::Error::last_os_error())
		} else {
			Ok(Self{
				mount_point: PathBuf::from(path)
			})
		}
	}
}

#[cfg(test)]
mod test {
	use super::super::TempDir;
	use super::RamFs;
	use std::fs::read_dir;
	use std::fs::File;

	// Test mounting and unmounting RamFs
	#[test]
	fn mount_unmount() {
		let mut tmp = TempDir::new("test").unwrap();

		assert_eq!(read_dir(&tmp).unwrap().count(), 0, "Mountpoint not empty");

		// Enter new scope to test unmounting
		{
			let _ramfs = RamFs::new("testfs", tmp.as_ref());

			assert_eq!(read_dir(&tmp).unwrap().count(), 0, "RamFs not empty after mounting");

			File::create(tmp.create_file("test").unwrap()).unwrap();

			assert_eq!(read_dir(&tmp).unwrap().count(), 1, "Clould not find created test file");
		}

		assert_eq!(read_dir(&tmp).unwrap().count(), 0, "Mountpoint not empty after unmount");
	}
}