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

pub type UId = libc::uid_t;

pub fn resolve_uid(user_name: &str) -> Result<UId, error::Error> {
	let c_user_name = CString::new(user_name)?;
	let mut user_info: libc::passwd = unsafe { std::mem::zeroed() };;
	let mut result: *mut libc::passwd = std::ptr::null_mut();

	// Dertermin the necessary buffer size for the getpwnam call.
	// If the size could not be determined, use 16 K
	let buffer_size = unsafe { libc::sysconf(libc::_SC_GETPW_R_SIZE_MAX) }.max(16535) as usize;
	let mut buffer = Vec::with_capacity(buffer_size);

	let error = unsafe { libc::getpwnam_r(c_user_name.as_ptr(), &mut user_info, buffer.as_mut_slice().as_mut_ptr(), buffer_size, &mut result) };
	if error == 0 {
		if result.is_null() {
			Err(Error::UserNotFound(String::from(user_name)))
		} else {
			Ok(user_info.pw_uid)
		}
	} else {
		Err(Error::OsError(io::Error::from_raw_os_error(error)))
	}
}

pub type GId = libc::gid_t;

pub fn resolve_gid(group_name: &str) -> Result<GId, error::Error> {
	let c_group_name = CString::new(group_name)?;
	let mut group_info: libc::group = unsafe { std::mem::zeroed() };;
	let mut result: *mut libc::group = std::ptr::null_mut();

	// Dertermin the necessary buffer size for the getpwnam call.
	// If the size could not be determined, use 16 K
	let buffer_size = unsafe { libc::sysconf(libc::_SC_GETPW_R_SIZE_MAX) }.max(16535) as usize;
	let mut buffer = Vec::with_capacity(buffer_size);

	let error = unsafe { libc::getgrnam_r(c_group_name.as_ptr(), &mut group_info, buffer.as_mut_slice().as_mut_ptr(), buffer_size, &mut result) };
	if error == 0 {
		if result.is_null() {
			Err(Error::GroupNotFound(String::from(group_name)))
		} else {
			Ok(group_info.gr_gid)
		}
	} else {
		Err(Error::OsError(io::Error::from_raw_os_error(error)))
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn res_uid_ok() {
		// Check that the uid is correctly resolved
		assert_eq!(resolve_uid("root").unwrap(), 0);
		assert_eq!(resolve_uid("bin").unwrap(), 1);
	}

	#[test]
	fn res_gid_ok() {
		// Check that the uid is correctly resolved
		assert_eq!(resolve_gid("root").unwrap(), 0);
		assert_eq!(resolve_gid("bin").unwrap(), 1);
	}
}