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

#[cfg(test)]
mod test {
	use super::*;
	use super::super::TempDir;
	use libc;
	use std::ffi::CString;
	use std::fs::File;
	use std::path::Path;
	use std::os::linux::fs::MetadataExt;

	fn create_test_file(file: &Path, owner: u32, group: u32, mode: u32 ) {
		let c_file = CString::new(file.to_str().unwrap()).unwrap();

		File::create(file).unwrap();

		unsafe {
			assert_eq!(libc::chmod(c_file.as_ptr(), mode), 0);
			assert_eq!(libc::chown(c_file.as_ptr(), owner, group), 0);
		}
	}

	fn copy_test_file(tmp: &mut TempDir, src_uid:u32, src_gid:u32, src_mode:u32, dst_uid:u32, dst_gid:u32, dst_mode:u32) {
		let file_s = tmp.create_file("source").unwrap();
		let file_d = tmp.create_file("destination").unwrap();

		create_test_file(&file_s, src_uid, src_gid, src_mode);
		create_test_file(&file_d, dst_uid, dst_gid, dst_mode);

		assert!(copy_perms_and_owners(&file_s, &file_d).is_ok());

		let file_d_md = file_d.metadata().unwrap();

		assert_eq!(file_d_md.st_uid(), src_uid, "Copying owner user ID failed");
		assert_eq!(file_d_md.st_gid(), src_gid, "Copying owner group ID failed");
		assert_eq!(file_d_md.st_mode() & 0o7777, src_mode & 0o7777, "Copying mode bits failed");
	}

	// Test copying of file permissions
	#[test]
	fn copy_test() {
		let mut tmp = TempDir::new("test").unwrap();

		copy_test_file(&mut tmp, 0, 0, 0o700, 1, 1, 0o555);
		copy_test_file(&mut tmp, 1000, 1100, 0o1241, 0, 0, 0o777);
	}
}