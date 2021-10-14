use anyhow::Result;
use fs_extra::dir::CopyOptions;
use std::env;

fn main() -> Result<()> {
	// This tells cargo to rerun this script if something in /assets/ changes.
	println!("cargo:rerun-if-changed=assets/*");

	let out_dir = env::var("OUT_DIR")?;

	let mut copy_options = CopyOptions::new();
	copy_options.overwrite = true;

	fs_extra::copy_items(&["assets/"], out_dir, &copy_options)?;

	Ok(())
}
