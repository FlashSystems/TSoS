use libc;
use log::{debug};
use std::path::Path;
use std::ffi::CString;
use std::io;
use std::os::linux::fs::MetadataExt;

// Until RFC 1861 lands we have to define the opaque ErrorContext type as a type
// with one zero size member.
#[cfg(feature = "acl")]
#[repr(C)]
struct ErrorContext { _private: [u8; 0] }

#[cfg(feature = "acl")]
#[link(name = "acl")]
extern {
	fn perm_copy_file (source: *const libc::c_char, destination: *const libc::c_char, ctx: *mut ErrorContext) -> libc::c_int;
}

#[cfg(feature = "acl")]
fn copy_permissions(src: &Path, _: &dyn MetadataExt, dst: &Path) -> io::Result<()> {
	let c_src = CString::new(src.to_str().unwrap())?;
	let c_dst = CString::new(dst.to_str().unwrap())?;

	debug!("Copying file ACLs from {} to {}...", src.display(), dst.display());

	let result = unsafe {
		let mut ctx: ErrorContext = std::mem::uninitialized();
		perm_copy_file(c_src.as_ptr(), c_dst.as_ptr(), &mut ctx)
	};

	if result == 0 {
		Ok(())
	} else {
		Err(io::Error::last_os_error())
	}
}

#[cfg(not(feature = "acl"))]
fn copy_permissions(src: &Path, src_metadata: &dyn MetadataExt, dst: &Path) -> io::Result<()> {
	let c_dst = CString::new(dst.to_str().unwrap())?;

	debug!("Copying file mode bits from {} to {}...", src.display(), dst.display());

	if unsafe { libc::chmod(c_dst.as_ptr(), src_metadata.st_mode()) } == 0 {
		Ok(())
	} else {
		Err(io::Error::last_os_error())
	}	
}

pub fn copy_perms_and_owners(src: &Path, dst: &Path) -> io::Result<()>{
	let c_dst = CString::new(dst.to_str().unwrap())?;
	let mdata = src.metadata()?;

	copy_permissions(&src, &mdata, &dst)?;

	debug!("Copying ownership information from {} => {}...", src.display(), dst.display());

	
	if unsafe { libc::chown(c_dst.as_ptr(), mdata.st_uid(), mdata.st_gid()) } == 0 {
		Ok(())
	} else {
		Err(io::Error::last_os_error())
	}
}