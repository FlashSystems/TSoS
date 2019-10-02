use std::io::Write;
use std::path::PathBuf;
use std::fs::File;
use std::thread::sleep;
use std::time::Duration;
use std::process::Command;
use temp_testdir::TempDir;

const TSOS_FILE: &str = "./target/debug/tsos";

fn to_file(tmp: &TempDir, file_name: &str, content: &str) -> PathBuf {
	let mut out_file_name = PathBuf::from(tmp.as_ref());
	out_file_name.push(file_name);

	let mut out = File::create(&out_file_name).unwrap();
	out.write_all(content.as_bytes()).unwrap();

	out_file_name
}

// Test that the process is startet with the correct uid and the default group id
#[test]
fn toml_uid_default() {
	let tmp = TempDir::default();

	let toml_file = to_file(&tmp, "uid.toml", r#"
		exec = "/usr/bin/id"
		uid = "bin"

		[secrets]
	"#);
	
	let output = Command::new(TSOS_FILE).arg(&toml_file).arg("-u").output().unwrap();
	assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "1");

	let output = Command::new(TSOS_FILE).arg(&toml_file).arg("-g").output().unwrap();
	assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "1");
}

// Test that the process is startet with the correct group id if only the group id is set.
#[test]
fn toml_gid() {
	let tmp = TempDir::default();

	let toml_file = to_file(&tmp, "uid.toml", r#"
		exec = "/usr/bin/id"
		gid = "bin"

		[secrets]
	"#);

	let output = Command::new(TSOS_FILE).arg(&toml_file).arg("-u").output().unwrap();
	assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "0");

	let output = Command::new(TSOS_FILE).arg(&toml_file).arg("-g").output().unwrap();
	assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "1");
}

// Test that the process is startet with the corret uid and gid if both are set.
#[test]
fn toml_uid_gid() {
	let tmp = TempDir::default();

	let toml_file = to_file(&tmp, "uid.toml", r#"
		exec = "/usr/bin/id"
		uid = "bin"
		gid = "root"

		[secrets]
	"#);

	let output = Command::new(TSOS_FILE).arg(&toml_file).arg("-u").output().unwrap();
	assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "1");

	let output = Command::new(TSOS_FILE).arg(&toml_file).arg("-g").output().unwrap();
	assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "0");
}

/// Test that the local search order of the providers is correct.
/// The local search order is the order defined by the search_path directive within
/// the TOML file.
#[test]
fn provider_local_search() {
	let tmp = TempDir::default();

	// Try provider search oder a -> b
	{
		let source = to_file(&tmp, "source.conf", "s1");

		let toml = format!(r#"
			#!/home/dgoss/Entwicklung/tsos/target/debug/tsos
			exec = "/usr/bin/cat"
			search_path = [ "./tests/providers/a", "./tests/providers/b" ]

			[secrets]
			provider =  [ "{}" ]
		"#, source.to_string_lossy());

		let toml_file = to_file(&tmp, "test.toml", &toml);

		let output = Command::new(TSOS_FILE).arg(toml_file).arg(source).output().unwrap();

		assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "s1:./tests/providers/a/provider");
	}

	// Try provider search oder b -> a
	{
		let source = to_file(&tmp, "source.conf", "s1");

		let toml = format!(r#"
			#!/home/dgoss/Entwicklung/tsos/target/debug/tsos
			exec = "/usr/bin/cat"
			search_path = [ "./tests/providers/b", "./tests/providers/a" ]

			[secrets]
			provider =  [ "{}" ]
		"#, source.to_string_lossy());

		let toml_file = to_file(&tmp, "test.toml", &toml);

		let output = Command::new(TSOS_FILE).arg(toml_file).arg(source).output().unwrap();

		assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "s1:./tests/providers/b/provider");
	}
}

/// Verify that the local provider is searched before the environment.
#[test]
fn provider_local_before_env_search() {
	let tmp = TempDir::default();

	let source = to_file(&tmp, "source.conf", "s1");

	let toml = format!(r#"
		#!/home/dgoss/Entwicklung/tsos/target/debug/tsos
		exec = "/usr/bin/cat"
		search_path = [ "./tests/providers/a" ]

		[secrets]
		provider =  [ "{}" ]
	"#, source.to_string_lossy());

	let toml_file = to_file(&tmp, "test.toml", &toml);

	let output = Command::new(TSOS_FILE).arg(toml_file).arg(source)
		.env("TSOS_PATH", "./tests/providers/b")
		.output().unwrap();

	assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "s1:./tests/providers/a/provider");
}

/// Verify that the environment is also searched.
#[test]
fn provider_env_search() {
	let tmp = TempDir::default();

	let source = to_file(&tmp, "source.conf", "s1");

	let toml = format!(r#"
		#!/home/dgoss/Entwicklung/tsos/target/debug/tsos
		exec = "/usr/bin/cat"
		search_path = [ "./tests/providers/a" ]

		[secrets]
		provider_ba =  [ "{}" ]
	"#, source.to_string_lossy());

	let toml_file = to_file(&tmp, "test.toml", &toml);

	let output = Command::new(TSOS_FILE).arg(toml_file).arg(source)
		.env("TSOS_PATH", "./tests/providers/b")
		.output().unwrap();

	assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "s1:./tests/providers/b/provider_ba");
}

/// Test that no mounts are leaking outside our TSOS container.
/// The test compares the output of the `mount` command before TSOS is startet with
/// the output of the same command while TSOS is running. If the output differs we
/// assume that we leaked a mount outside of the process.
#[test]
fn mount_leakage() {
	let tmp = TempDir::default();

	let source1 = to_file(&tmp, "source1.conf", "s1");
	let source2 = to_file(&tmp, "source2.conf", "s2");

	let toml = format!(r#"
		#!/home/dgoss/Entwicklung/tsos/target/debug/tsos
		exec = "/usr/bin/sleep"
		search_path = [ "./tests/providers" ]

		[secrets]
		provider =  [ "{}", "{}" ]
	"#, source1.to_string_lossy(), source2.to_string_lossy());

	let toml_file = to_file(&tmp, "test.toml", &toml);

	let mount_before = Command::new("/usr/bin/mount").output().unwrap();
	let mount_before = String::from_utf8_lossy(&mount_before.stdout);

	// Spawn the child process and wait 2 seconds for it to setup its mounts.
	let mut child = Command::new(TSOS_FILE).arg(toml_file).arg("4").spawn().unwrap();
	sleep(Duration::from_secs(2));

	// Now check for leaked mounts while the child is still running.
	let mount_during = Command::new("/usr/bin/mount").output().unwrap();
	let mount_during = String::from_utf8_lossy(&mount_during.stdout);

	assert!(child.wait().unwrap().success());

	// While TSOS is running the outside world may not see any difference in its mounts.
	assert_eq!(mount_before, mount_during, "A mount leaked outside the tsos process.");
}

/// Check that the mounts inside the TSOS container are differnet from the mounts outside.
/// This is the accompanying check to mount_leakage.
#[test]
fn mount_inside() {
	let tmp = TempDir::default();

	let source = to_file(&tmp, "source.conf", "s1");

	let toml = format!(r#"
		#!/home/dgoss/Entwicklung/tsos/target/debug/tsos
		exec = "/usr/bin/mount"
		search_path = [ "./tests/providers" ]

		[secrets]
		provider =  [ "{}" ]
	"#, source.to_string_lossy());

	let toml_file = to_file(&tmp, "test.toml", &toml);

	let mount_before = Command::new("/usr/bin/mount").output().unwrap();
	let mount_before = String::from_utf8_lossy(&mount_before.stdout);

	let mount_child = Command::new(TSOS_FILE).arg(toml_file).output().unwrap();
	let mount_child = String::from_utf8_lossy(&mount_child.stdout);

	// The output of the mount command must differ between the TSOS process and
	// the outside world.
	assert_ne!(mount_before, mount_child);
}

/// This test verifies that the output of the provider is correctly overlayed
/// over the source file and that multiple providers are correctly executed.
#[test]
fn multiple_providers() {
	let tmp = TempDir::default();

	let source1 = to_file(&tmp, "source1.conf", "s1");
	let source2 = to_file(&tmp, "source2.conf", "s2");
	let source3 = to_file(&tmp, "source3.conf", "s3");

	let toml = format!(r#"
		#!/home/dgoss/Entwicklung/tsos/target/debug/tsos
		exec = "/usr/bin/cat"
		search_path = [ "./tests/providers/a", "./tests/providers/b" ]

		[secrets]
		provider_aa = [ "{}", "{}" ]
		provider_ba = [ "{}" ]
	"#, source1.to_string_lossy(), source2.to_string_lossy(), source3.to_string_lossy());

	let toml_file = to_file(&tmp, "test.toml", &toml);

	// Spawn the child process and wait 2 seconds for it to setup its mounts.
	let output = Command::new(TSOS_FILE).arg(toml_file)
		.arg(source1)
		.arg(source2)
		.arg(source3)
		.output().unwrap();
	
	assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "s1:./tests/providers/a/provider_aa\ns2:./tests/providers/a/provider_aa\ns3:./tests/providers/b/provider_ba");
}
