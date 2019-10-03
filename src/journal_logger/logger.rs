use log::{Record, Level, Metadata, SetLoggerError};
use std::ffi::CString;
use libc;

#[link(name = "systemd")]
extern "C" {
	pub fn sd_journal_print(priority: libc::c_int, format: * const libc::c_char , ... ) -> libc::c_int;
}

/* from syslog.h */
#[allow(dead_code)]
pub enum Priority {
	Emerg =   0, /* system is unusable */
	Alert =   1, /* action must be taken immediately */
	Crit =    2, /* critical conditions */
	Error =   3, /* error conditions */
	Warning = 4, /* warning conditions */
	Notice =  5, /* normal but significant condition */
	Info =    6, /* informational */
	Debug =   7  /* debug-level messages */
}

/// Implements a systemd logger
pub struct JournalLogger {
	level: Level
}

impl log::Log for JournalLogger {
	fn enabled(&self, metadata: &Metadata) -> bool {
		metadata.level() <= self.level
	}

	fn log(&self, record: &Record) {
		if self.enabled(record.metadata()) {
			let priority = match record.metadata().level() {
				Level::Debug => Priority::Debug,
				Level::Trace => Priority::Debug,
				Level::Info => Priority::Info,
				Level::Warn => Priority::Warning,
				Level::Error => Priority::Error
			};

			// Convert the parameters into NULL terminated strings
			let c_format_string = CString::new("%s").unwrap();
			let c_message = CString::new(format!("{}", record.args())).expect("Conversion to C string failed.");

			unsafe {
					sd_journal_print(priority as libc::c_int, c_format_string.as_ptr(), c_message.as_ptr());
			}
		}
	}

	fn flush(&self) {
	}
}

pub fn init_with_level(level: Level) -> Result<(), SetLoggerError> {
	log::set_boxed_logger(Box::new(JournalLogger {
		level
	}))?;

	log::set_max_level(level.to_level_filter());

	Ok(())
}