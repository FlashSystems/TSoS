use std::io::BufRead;
use std::process::Command;
use std::io::Cursor;
use log::{Level, info};
use std::{fs, env};
use std::os::unix::fs::MetadataExt;
use std::collections::HashMap;
use std::time::SystemTime;

extern crate journal_logger;

/// Verify that the journal detection using the JOURNAL_STREAM environment variable works.
/// This is done by fabricating a JOURNAL_STREAM environment variable that contains
/// the device and inode of stdout and stderr.
#[test]
fn has_journal() {
	let meta = fs::metadata("/dev/stderr").unwrap();
	env::set_var("JOURNAL_STREAM", format!("{}:{}", meta.dev(), meta.ino()));
	assert!(journal_logger::has_journal());

	let meta = fs::metadata("/dev/stdout").unwrap();
	env::set_var("JOURNAL_STREAM", format!("{}:{}", meta.dev(), meta.ino()));
	assert!(journal_logger::has_journal());

	// Check that only matching the inode number does not work
	env::set_var("JOURNAL_STREAM", format!("{}:{}", meta.dev()+1, meta.ino()));
	assert!(!journal_logger::has_journal());

	// Check that only matching the device number does not work
	env::set_var("JOURNAL_STREAM", format!("{}:{}", meta.dev(), meta.ino()+1));
	assert!(!journal_logger::has_journal());
}

/// Test the journal logger by wrting a journal entry and using journalctl
/// to read the entry back. The fields of the entry are checked if they contain
/// the correct values.
#[test]
fn write_log() {
	let start_time = SystemTime::now();

	assert!(journal_logger::init_with_level(Level::Debug).is_ok());

	// Log a line and save the line this has been done for later reference within the assert.
	info!(target: "journal_logger_test", "Testing the journal_logger crate."); let log_line = line!();

	// Read the journal from the start timestamp of this test onward and extract the text journal entry
	let output = Command::new("journalctl")
	.arg("-le")
	.arg(format!("-S@{}", start_time.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()))
	.arg("-tjournal_logger_test")
	.arg("-oexport")
	.output().unwrap();

	// Parse the journal entry and create a HashMap for checking the values.
	let mut stdout = Cursor::new(output.stdout);
	let mut buffer = String::new();
	let mut values = HashMap::new();
	while let Ok(len) = stdout.read_line(&mut buffer) {
		// Exit on eof
		if len == 0 { break }
		let key_value = buffer.trim().splitn(2, '=');

		if let [key, value] = key_value.collect::<Vec<&str>>()[0..] {
			values.insert(String::from(key), String::from(value));
		}

		// Clear the buffer for the next round
		buffer.clear();
	}

	// Verify the content of the journal entry
	assert_eq!(values.get("MESSAGE").unwrap(), "Testing the journal_logger crate.", "Wrong MESSAGE content.");
	assert_eq!(values.get("PRIORITY").unwrap(), "6", "Wrong PRIORITY field.");
	assert_eq!(values.get("_TRANSPORT").unwrap(), "journal", "Message transport was not journal.");
	assert_eq!(values.get("MODULE_PATH").unwrap(), "lib", "Module path not 'lib'.");
	assert_eq!(values.get("CODE_LINE").unwrap(), &format!("{}", log_line), "Wrong CODE_LINE field.");
	assert_eq!(values.get("CODE_FILE").unwrap(), file!(), "Wrong CODE_FILE field.");
	assert_eq!(values.get("_PID").unwrap(), &format!("{}", std::process::id()), "Journal _PID field does not contain test process PID.");
	assert_eq!(values.get("SYSLOG_IDENTIFIER").unwrap(), "journal_logger_test", "Wrong SYSLOG_IDENTIFIER field.");
}