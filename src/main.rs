use std::path::PathBuf;
use std::error;

mod system;
use system::TempDir;

mod config;
use config::Config;

fn go(config: &Config) -> Result<(), Box<dyn error::Error>> {
	system::unshare_mount_ns()?;

	// Create temporary directory and mount a ramfs onto it
	let temp = TempDir::new("tsos")?;
	system::mount_ramfs(512, "tsos", temp.as_ref())?;


	system::umount(temp.as_ref());

	Ok(())
}

fn main() {
	let config = Config::new(&PathBuf::from("./test/myprog.toml"));

	go(&config.unwrap()).unwrap();


	//system::spawn_wait(&PathBuf::from("/usr/bin/sleep"), &vec!("10")).expect("spawn_wait");
	//system::spawn_wait(&PathBuf::from("/usr/bin/echo"), &vec!("-e", "asd")).expect("spawn_wait");


	print!("DONE"); 
}
