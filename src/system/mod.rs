use libc;
use std::path::Path;
use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;
use std::io;

mod tempdir;
mod error;

pub use tempdir::TempDir;
pub use error::Error;

pub fn unshare_mount_ns() -> io::Result<()> {
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

	if unsafe { libc::mount(c_source_tag.as_ptr(), c_path.as_ptr(), c_fstype.as_ptr(), libc::MS_NODEV|libc::MS_NOEXEC, c_params.as_ptr() as *const libc::c_void) } < 0 {
		return Err(io::Error::last_os_error());
	}

	// Make the mount private. We don't want this mount point to propagate anywhere.
	if unsafe { libc::mount(ptr::null(), c_path.as_ptr(), ptr::null(), libc::MS_PRIVATE, ptr::null()) } < 0 {
		Err(io::Error::last_os_error())
	} else {
		Ok(())
	}
}

pub fn umount(path: &Path) -> io::Result<()> {
	let c_path = CString::new(path.to_str().unwrap())?;

	if unsafe { libc::umount(c_path.as_ptr()) } < 0 {
		Err(io::Error::last_os_error())
	} else {
		Ok(())
	}
	
}

pub fn spawn_wait(script: &Path, args: &Vec<&str>) -> Result<(), Error> {
	let child_pid = unsafe { libc::fork() };

	if child_pid < 0 {
		Err(Error::OsError(io::Error::last_os_error()))
	} else if child_pid == 0 {
		let c_script = CString::new(script.to_str().unwrap())?;

		// The argument array must be null-Terminated
		let script_file_name = script.file_name().ok_or(Error::InvalidArgument("script"))?.to_str().ok_or(Error::InvalidArgument("script"))?;

		// Prepare a vector of CString instances for the argument list
		let mut args_array = Vec::<Option<CString>>::with_capacity(args.len() + 2);
		args_array.push(Some(CString::new(script_file_name)?));
		for &arg in args {
			args_array.push(Some(CString::new(arg)?));
		}
		args_array.push(None);

		// Convert the vector of instances into a vector of pointers
		let pargs_array: Vec::<*const c_char> = args_array.iter().map(|arg| if let Some(a) = arg { a.as_ptr() } else { ptr::null() }).collect();

		unsafe { libc::execv(c_script.as_ptr(), pargs_array.as_ptr()) };
		Ok(())
	} else {
		let mut status = 0;
		unsafe {
			libc::waitpid(child_pid, &mut status, libc::WEXITED);

			let exit_status = libc::WEXITSTATUS(status);
			if ( ! libc::WIFEXITED(status)) || (exit_status != 0) {
				Err(Error::ExecFailed(exit_status))
			} else {
				Ok(())
			}
		}
	}
}