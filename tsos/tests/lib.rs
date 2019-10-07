use std::io::Write;
use std::path::PathBuf;
use std::fs::File;
use std::thread::sleep;
use std::time::Duration;
use std::process::Command;
use temp_testdir::TempDir;

// Arguments to pass to cargo to run the tsos executable
const CARGO_ARGS: &[&str] = &[ "run", "-q", "--" ];

// Path to different binaries required for the tests
const BIN_CAT: &str = "/usr/bin/cat";
const BIN_ID: &str = "/usr/bin/id";
const BIN_SLEEP: &str = "/usr/bin/sleep";
const BIN_MOUNT: &str = "/usr/bin/mount";

const TEST_USER: &str = "nobody";
const TEST_GROUP: &str = "nobody";

// Path to the secret providers used for tesing
const PROV_PATH: &str = "./tests/providers";

/// Writes the content of a string into a temporary files inside a TempDir.
fn to_file(tmp: &TempDir, file_name: &str, content: &str) -> PathBuf {
	let mut out_file_name = PathBuf::from(tmp.as_ref());
	out_file_name.push(file_name);

	let mut out = File::create(&out_file_name).unwrap();
	out.write_all(content.as_bytes()).unwrap();

	out_file_name
}

/// Uses the `id` to resolve a user name into a user id.
/// Using the `id` command makes sure that the reference value is correct.
fn resolve_uid(user_name: &str) -> u32 {
	let output = Command::new(BIN_ID)
		.arg("-u")
		.arg(user_name)
		.output().unwrap();

	u32::from_str_radix(String::from_utf8_lossy(&output.stdout).trim(), 10).unwrap()
}

/// Uses the `id` to resolve a group name into a group id.
/// Using the `id` command makes sure that the reference value is correct.
fn resolve_gid(user_name: &str) -> u32 {
	let output = Command::new(BIN_ID)
		.arg("-g")
		.arg(user_name)
		.output().unwrap();

	u32::from_str_radix(String::from_utf8_lossy(&output.stdout).trim(), 10).unwrap()
}

/// Test that the process is startet with the correct uid and the default group id
#[test]
fn toml_uid_default() {
	let tmp = TempDir::default();

	let toml_file = to_file(&tmp, "uid.toml", &format!(r#"
		exec = "{bin}"
		uid = "{username}"

		[secrets]
	"#, bin = BIN_ID, username = TEST_USER));
	
	let output = Command::new("cargo").args(CARGO_ARGS)
		.arg(&toml_file)
		.arg("-u")
		.output().unwrap();
	assert_eq!(u32::from_str_radix(String::from_utf8_lossy(&output.stdout).trim(), 10).unwrap(), resolve_uid(TEST_USER));

	let output = Command::new("cargo").args(CARGO_ARGS)
		.arg(&toml_file)
		.arg("-g")
		.output().unwrap();
	assert_eq!(u32::from_str_radix(String::from_utf8_lossy(&output.stdout).trim(), 10).unwrap(), resolve_gid(TEST_USER));
}

/// Test that the process is startet with the correct group id if only the group id is set.
#[test]
fn toml_gid() {
	let tmp = TempDir::default();

	let toml_file = to_file(&tmp, "uid.toml", &format!(r#"
		exec = "{bin}"
		gid = "{groupname}"

		[secrets]
	"#, bin = BIN_ID, groupname = TEST_GROUP));

	let output = Command::new("cargo").args(CARGO_ARGS)
		.arg(&toml_file)
		.arg("-u")
		.output().unwrap();
	assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "0");

	let output = Command::new("cargo").args(CARGO_ARGS)
		.arg(&toml_file)
		.arg("-g")
		.output().unwrap();
	assert_eq!(u32::from_str_radix(String::from_utf8_lossy(&output.stdout).trim(), 10).unwrap(), resolve_gid(TEST_GROUP));
}

/// Test that the process is startet with the corret uid and gid if both are set.
#[test]
fn toml_uid_gid() {
	let tmp = TempDir::default();

	let toml_file = to_file(&tmp, "uid.toml", &format!(r#"
		exec = "{bin}"
		uid = "{username}"
		gid = "{groupname}"

		[secrets]
	"#, bin = BIN_ID, username = TEST_USER, groupname = TEST_GROUP));

	let output = Command::new("cargo").args(CARGO_ARGS)
		.arg(&toml_file)
		.arg("-u")
		.output().unwrap();
	assert_eq!(u32::from_str_radix(String::from_utf8_lossy(&output.stdout).trim(), 10).unwrap(), resolve_uid(TEST_USER));

	let output = Command::new("cargo").args(CARGO_ARGS)
		.arg(&toml_file)
		.arg("-g")
		.output().unwrap();
	assert_eq!(u32::from_str_radix(String::from_utf8_lossy(&output.stdout).trim(), 10).unwrap(), resolve_gid(TEST_GROUP));
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

		let toml_file = to_file(&tmp, "test.toml", &format!(r#"
			exec = "{bin}"
			search_path = [ "{path}/a", "{path}/b" ]

			[secrets]
			provider =  [ "{source}" ]
		"#, bin = BIN_CAT, path = PROV_PATH, source = source.to_string_lossy()));

		let output = Command::new("cargo").args(CARGO_ARGS)
			.arg(toml_file)
			.arg(source)
			.output().unwrap();

		assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), format!("s1:{}/a/provider", PROV_PATH));
	}

	// Try provider search oder b -> a
	{
		let source = to_file(&tmp, "source.conf", "s1");

		let toml_file = to_file(&tmp, "test.toml", &format!(r#"
			exec = "{bin}"
			search_path = [ "{path}/b", "{path}/a" ]

			[secrets]
			provider =  [ "{source}" ]
		"#, bin = BIN_CAT, path = PROV_PATH, source = source.to_string_lossy()));

		let output = Command::new("cargo").args(CARGO_ARGS)
			.arg(toml_file)
			.arg(source)
			.output().unwrap();

		assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), format!("s1:{}/b/provider", PROV_PATH));
	}
}

/// Verify that the local provider is searched before the environment.
#[test]
fn provider_local_before_env_search() {
	let tmp = TempDir::default();

	let source = to_file(&tmp, "source.conf", "s1");

	let toml_file = to_file(&tmp, "test.toml", &format!(r#"
		exec = "{bin}"
		search_path = [ "{path}/a" ]

		[secrets]
		provider =  [ "{source}" ]
	"#, bin = BIN_CAT, path = PROV_PATH, source = source.to_string_lossy()));

	let output = Command::new("cargo").args(CARGO_ARGS)
		.arg(toml_file)
		.arg(source)
		.env("TSOS_PATH", format!("{}/b", PROV_PATH))
		.output().unwrap();

	assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), format!("s1:{}/a/provider", PROV_PATH));
}

/// Verify that the environment is also searched.
#[test]
fn provider_env_search() {
	let tmp = TempDir::default();

	let source = to_file(&tmp, "source.conf", "s1");

	let toml_file = to_file(&tmp, "test.toml", &format!(r#"
		exec = "{bin}"
		search_path = [ "{path}/a" ]
		env_path = true

		[secrets]
		provider_b =  [ "{source}" ]
	"#, bin = BIN_CAT, path = PROV_PATH, source = source.to_string_lossy()));

	let output = Command::new("cargo").args(CARGO_ARGS)
		.arg(toml_file)
		.arg(source)
		.env("TSOS_PATH", format!("{}/b", PROV_PATH))
		.output().unwrap();

	assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), format!("s1:{}/b/provider_b", PROV_PATH));
}

/// Verify that the environment is not searched if the
/// env_path configuration option is missing.
#[test]
fn provider_env_search_off() {
	let tmp = TempDir::default();

	let source = to_file(&tmp, "source.conf", "s1");

	let toml_file = to_file(&tmp, "test.toml", &format!(r#"
		exec = "{bin}"
		search_path = [ "{path}/a" ]

		[secrets]
		provider_b =  [ "{source}" ]
	"#, bin = BIN_CAT, path = PROV_PATH, source = source.to_string_lossy()));

	let output = Command::new("cargo").args(CARGO_ARGS)
		.arg(toml_file)
		.arg(source)
		.env("TSOS_PATH", format!("{}/b", PROV_PATH))
		.output().unwrap();

	assert!(!output.status.success(), "Provider from env was found but should not have been.");
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

	let toml_file = to_file(&tmp, "test.toml", &format!(r#"
		exec = "{bin}"
		search_path = [ "{path}" ]

		[secrets]
		provider =  [ "{source1}", "{source2}" ]
	"#, bin = BIN_SLEEP, path = PROV_PATH, source1 = source1.to_string_lossy(), source2 = source2.to_string_lossy()));

	let mount_before = Command::new(BIN_MOUNT).output().unwrap();
	let mount_before = String::from_utf8_lossy(&mount_before.stdout);

	// Spawn the child process and wait 2 seconds for it to setup its mounts.
	let mut child = Command::new("cargo").args(CARGO_ARGS)
		.arg(toml_file)
		.arg("4")
		.spawn().unwrap();
	sleep(Duration::from_secs(2));

	// Now check for leaked mounts while the child is still running.
	let mount_during = Command::new(BIN_MOUNT).output().unwrap();
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

	let toml_file = to_file(&tmp, "test.toml", &format!(r#"
		exec = "{bin}"
		search_path = [ "{path}" ]

		[secrets]
		provider =  [ "{source}" ]
	"#, bin = BIN_MOUNT, path = PROV_PATH, source = source.to_string_lossy()));

	let mount_before = Command::new(BIN_MOUNT).output().unwrap();
	let mount_before = String::from_utf8_lossy(&mount_before.stdout);

	let mount_child = Command::new("cargo").args(CARGO_ARGS)
		.arg(toml_file)
		.output().unwrap();
	let mount_child = String::from_utf8_lossy(&mount_child.stdout);

	// The output of the mount command must differ between the TSOS process and
	// the outside world.
	assert_ne!(mount_before, mount_child);
}

/// This test verifies that the output of asingle provider is correctly overlayed
/// over the source file.
#[test]
fn single_provider() {
	let tmp = TempDir::default();

	let source = to_file(&tmp, "source1.conf", "s1");

	let toml_file = to_file(&tmp, "test.toml", &format!(r#"
		exec = "{bin}"
		search_path = [ "{path}" ]

		[secrets]
		provider = [ "{source}" ]
	"#, bin = BIN_CAT, path = PROV_PATH, source = source.to_string_lossy()));

	// Spawn the child process and wait 2 seconds for it to setup its mounts.
	let output = Command::new("cargo").args(CARGO_ARGS)
		.arg(toml_file)
		.arg(source)
		.output().unwrap();
	
	assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), format!("s1:{path}/provider", path = PROV_PATH));
}

/// This test verifies that the output of the provider is correctly overlayed
/// over the source file and that multiple providers are correctly executed.
#[test]
fn multiple_providers() {
	let tmp = TempDir::default();

	let source1 = to_file(&tmp, "source1.conf", "s1");
	let source2 = to_file(&tmp, "source2.conf", "s2");
	let source3 = to_file(&tmp, "source3.conf", "s3");

	let toml_file = to_file(&tmp, "test.toml", &format!(r#"
		exec = "{bin}"
		search_path = [ "{path}/a", "{path}/b" ]

		[secrets]
		provider_a = [ "{source1}", "{source2}" ]
		provider_b = [ "{source3}" ]
	"#, bin = BIN_CAT, path = PROV_PATH, source1 = source1.to_string_lossy(), source2 = source2.to_string_lossy(), source3 = source3.to_string_lossy()));

	// Spawn the child process and wait 2 seconds for it to setup its mounts.
	let output = Command::new("cargo").args(CARGO_ARGS)
		.arg(toml_file)
		.arg(source1)
		.arg(source2)
		.arg(source3)
		.output().unwrap();
	
	assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), format!("s1:{path}/a/provider_a\ns2:{path}/a/provider_a\ns3:{path}/b/provider_b", path = PROV_PATH));
}

/// This test verifies that a missing template (source) file leads to an error.
#[test]
fn missing_source() {
	let tmp = TempDir::default();

	let mut inv_source = PathBuf::from(tmp.as_ref());
	inv_source.push("missing");


	let toml_file = to_file(&tmp, "test.toml", &format!(r#"
		exec = "{bin}"
		search_path = [ "{path}" ]

		[secrets]
		provider = [ "{source}" ]
	"#, bin = BIN_CAT, path = PROV_PATH, source = inv_source.to_string_lossy()));

	// Spawn the child process and wait 2 seconds for it to setup its mounts.
	let output = Command::new("cargo").args(CARGO_ARGS)
		.arg(toml_file)
		.arg(inv_source)
		.output().unwrap();
	
	assert!(!output.status.success());
}
