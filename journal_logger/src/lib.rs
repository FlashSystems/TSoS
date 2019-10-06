use std::env;
use libc;
use std::os::unix::io::{RawFd, AsRawFd};
use std::mem::MaybeUninit;
use std::io::{stdout, stderr};

#[cfg(target_os = "linux")]
mod logger;
#[cfg(target_os = "linux")]
pub use logger::init_with_level;

#[cfg(not(target_os = "linux"))]
pub fn init_with_level(level: Level) -> Result<(), SetLoggerError> {
	panic!("Called journal_logger::init_with_level on an unsupported platform. Use has_journal to prevent hat.");
}


/// Checks that the device and inode number of the passed RawFd match the 
/// values in device and inode. If an error occures false is returned.
#[cfg(target_os = "linux")]
fn check_descriptor(device: libc::dev_t, inode: libc::ino_t, fd: RawFd) -> bool{
	unsafe { 
		let mut stat = MaybeUninit::<libc::stat64>::zeroed();
		if libc::fstat64(fd, stat.as_mut_ptr()) == 0 {
			let stat = stat.assume_init();
			( stat.st_dev == device ) && ( stat.st_ino == inode )
		} else {
			false
		}
	}
}

/// Check if stdout or stderr are connected to the journal stream that was passed via the
/// JOURNAL_STREAM environment variable. If that's the case we return true and the
/// main program should upgrade to journald logging. If an error occures or stdout/stderr
/// are not connected to the journal we return false.
#[cfg(target_os = "linux")]
pub fn has_journal() -> bool {
	if let Ok(journal_stream) = env::var("JOURNAL_STREAM") {
		if let [Some(device), Some(inode)] = journal_stream.split(":").map(|v| u64::from_str_radix(v.trim(), 10).ok()).collect::<Vec<Option<u64>>>()[0..2] {
			// Check if stderr or stdout match the passed device and inode number
			check_descriptor(device, inode, stderr().as_raw_fd()) ||
			check_descriptor(device, inode, stdout().as_raw_fd())
		} else {
			false
		}
	} else {
		false
	}
}

/// If we're not on linux, we always return false because systemd is only available on linux.
#[cfg(not(target_os = "linux"))]
pub fn has_journal() -> bool {
	false
}