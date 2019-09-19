use libc;
use log::debug;
use std::path::Path;
use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;
use std::io;
use std::process;

mod tempdir;
mod error;

pub use tempdir::TempDir;
pub use error::Error;

pub fn unshare_mount_ns() -> io::Result<()> {
	debug!("Unshare mount namespaces...");

	if unsafe { libc::unshare(libc::CLONE_NEWNS) } < 0 {
		Err(io::Error::last_os_error())
	} else {
		Ok(())
	}
}

pub fn mount_ramfs(size: usize, source_tag: &str, path: &Path) -> io::Result<()> {
	let c_source_tag = CString::new(source_tag)?;
	let c_path = CString::new(path.to_str().unwrap())?;
	let c_fstype = CString::new("ramfs")?;
	let c_params = CString::new(format!("size={},mode=701", size))?;

	debug!("Mounting ramfs on {}...", path.display());

	if unsafe { libc::mount(c_source_tag.as_ptr(), c_path.as_ptr(), c_fstype.as_ptr(), libc::MS_NODEV|libc::MS_NOEXEC, c_params.as_ptr() as *const libc::c_void) } < 0 {
		return Err(io::Error::last_os_error());
	}

	debug!("Making mount {} private...", path.display());

	// Make the mount private. We don't want this mount point to propagate anywhere.
	if unsafe { libc::mount(ptr::null(), c_path.as_ptr(), ptr::null(), libc::MS_PRIVATE, ptr::null()) } < 0 {
		Err(io::Error::last_os_error())
	} else {
		Ok(())
	}
}

pub fn umount(path: &Path) -> io::Result<()> {
	let c_path = CString::new(path.to_str().unwrap())?;

	debug!("Unmounting {}...", path.display());

	if unsafe { libc::umount(c_path.as_ptr()) } < 0 {
		Err(io::Error::last_os_error())
	} else {
		Ok(())
	}
	
}