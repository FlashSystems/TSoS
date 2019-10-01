use std::process::Command;

const TSOS_FILE: &str = "./target/debug/tsos";
const TEST_CONFIG: &str = "./tests";

// Test that the process is startet with the correct uid and the default group id
#[test]
fn toml_uid_default() {
	let output = Command::new(TSOS_FILE).arg(TEST_CONFIG.to_owned() + "/uid.toml").arg("-u").output().unwrap();
	assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "1");

	let output = Command::new(TSOS_FILE).arg(TEST_CONFIG.to_owned() + "/uid.toml").arg("-g").output().unwrap();
	assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "1");
}

// Test that the process is startet with the correct group id if only the group id is set.
#[test]
fn toml_gid() {
	let output = Command::new(TSOS_FILE).arg(TEST_CONFIG.to_owned() + "/gid.toml").arg("-u").output().unwrap();
	assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "0");

	let output = Command::new(TSOS_FILE).arg(TEST_CONFIG.to_owned() + "/gid.toml").arg("-g").output().unwrap();
	assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "1");
}

// Test that the process is startet with the corret uid and gid if both are set.
#[test]
fn toml_uid_gid() {
	let output = Command::new(TSOS_FILE).arg(TEST_CONFIG.to_owned() + "/uid-gid.toml").arg("-u").output().unwrap();
	assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "1");

	let output = Command::new(TSOS_FILE).arg(TEST_CONFIG.to_owned() + "/uid-gid.toml").arg("-g").output().unwrap();
	assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "0");
}