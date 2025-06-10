use std::ffi::OsString;
use std::fs::File;
use std::io::{Read, Write};
use std::sync::LazyLock;

use anyhow::Result;
use chrono::Local;
use clap::Parser;
use env_logger::Builder;
#[cfg(target_os = "hermit")]
use hermit as _;
use hermit_wasm::run_preview1;
use log::{LevelFilter, info};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(name = "hermit-wasm")]
#[command(next_line_help = true)]
pub struct Config {
	/// The WebAssembly module to run and arguments to pass to it.
	#[arg(value_name = "WASM")]
	pub module_and_args: Vec<OsString>,
}

static CONFIG: LazyLock<Config> = LazyLock::new(Config::parse);

pub fn main() -> Result<()> {
	Builder::new()
		.filter_level(LevelFilter::Info)
		.parse_env("RUST_LOG")
		.format(|buf, record| {
			writeln!(
				buf,
				"[{} {}] {}",
				record.level(),
				//Format like you want to: <-----------------
				Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
				record.args()
			)
		})
		.init();

	// First step is to create the Wasm execution engine with some config.
	// Currently, we are using the default configuration.
	let config = wasmtime::Config::new();

	if CONFIG.module_and_args.is_empty() {
		eprintln!("No WebAssembly module specified. Please provide a .wasm file to run.");
		std::process::exit(1);
	}
	let fname = CONFIG.module_and_args[0].clone();
	info!("Start Hermit-WASM: {fname:?}");

	let mut buffer = Vec::new();
	let mut f = File::open(fname).expect("Unable to open wasm module");

	f.read_to_end(&mut buffer)?;

	run_preview1(buffer.as_slice(), &config, &CONFIG.module_and_args)
}
