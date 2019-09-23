use libc;
use log::debug;
use std::path::Path;
use std::ffi::CString;
use std::ptr;
use std::io;

mod tempdir;
mod error;
mod ramfs;
mod permissions;

pub use tempdir::TempDir;
pub use error::Error;
pub use ramfs::RamFs;
pub use permissions::copy_perms_and_owners;

pub fn bind(source: &Path, target: &Path) -> Result<(), error::Error> {
		let c_source = CString::new(source.to_str().unwrap())?;
		let c_target = CString::new(target.to_str().unwrap())?;

		debug!("Binding {} on {}...", source.display(), target.display());

		if unsafe { libc::mount(c_source.as_ptr(), c_target.as_ptr(), ptr::null(), libc::MS_BIND|libc::MS_PRIVATE, ptr::null()) } < 0 {
			Err(error::Error::OsError(io::Error::last_os_error()))
		} else {
			Ok(())
		}
}

pub fn unshare_mount_ns() -> io::Result<()> {
	debug!("Unshare mount namespaces...");

	if unsafe { libc::unshare(libc::CLONE_NEWNS) } != 0 {
		Err(io::Error::last_os_error())
	} else {
		debug!("Change root file system propagation to private...");
		
		// Now disable the forwarding of mount changes for all already present mount points.
		// This isolates this processes mount namespace completely from the parent process.
		if unsafe { libc::mount(CString::new("none")?.as_ptr(), CString::new("/")?.as_ptr(), ptr::null(), libc::MS_PRIVATE|libc::MS_REC, ptr::null()) } < 0 {
			Err(io::Error::last_os_error())
		} else {
			Ok(())
		}
	}
}
