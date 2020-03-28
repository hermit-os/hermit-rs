use std::env;
use std::process::Command;

#[cfg(not(feature = "rustc-dep-of-std"))]
fn build() {
	let out_dir = env::var("OUT_DIR").unwrap();
	let profile = env::var("PROFILE").expect("PROFILE was not set");

	let _output = Command::new("cargo")
		.current_dir(out_dir.clone())
		.arg("download")
		.arg("--output")
		.arg(out_dir.clone()+"/rusty-hermit")
		.arg("--extract")
		.arg("rusty-hermit")
		.output()
		.expect("Unable to download rusty-hermit. Please install `cargo-download`.");

	if profile == "release" {
		let _output = Command::new("cargo")
			.current_dir(out_dir.clone() + "/rusty-hermit")
			.arg("build")
			.arg("-Z")
			.arg("build-std=core,alloc")
			.arg("--target")
			.arg("x86_64-unknown-hermit-kernel")
			.arg("--release")
			.output()
			.expect("Unable to build kernel");
	} else {
		let _output = Command::new("cargo")
			.current_dir(out_dir.clone() + "/rusty-hermit")
			.arg("build")
			.arg("-Z")
			.arg("build-std=core,alloc")
			.arg("--target")
			.arg("x86_64-unknown-hermit-kernel")
			.output()
			.expect("Unable to build kernel");
	}

	println!(
		"cargo:rustc-link-search=native={}/rusty-hermit/target/x86_64-unknown-hermit-kernel/{}",
		out_dir.clone(),
		profile
	);
	println!("cargo:rustc-link-lib=static=hermit");
}

fn main() {
	#[cfg(not(feature = "rustc-dep-of-std"))]
	build();
}
