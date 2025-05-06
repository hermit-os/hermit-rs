#![allow(dependency_on_unit_never_type_fallback)]

use std::fs::File;
use std::io::{Read, Write};

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
#[command(next_line_help = true)]
pub struct Config {
	/// File name of the WASM module
	#[arg(short, long, value_name = "FILE")]
	fname: Option<String>,

	/// Defines the usage of the WebAssembly threads proposal for compilation
	#[arg(short, long, default_value_t = false)]
	threads: bool,
}

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

	let args = Config::parse();

	// First step is to create the Wasm execution engine with some config.
	// In this example we are using the default configuration.
	let mut config = wasmtime::Config::new();
	config.wasm_threads(args.threads);

	for argument in std::env::args() {
		println!("{argument}");
	}

	if let Some(fname) = args.fname {
		info!("Start Hermit-WASM!");

		let mut buffer = Vec::new();
		let mut f = File::open(fname)?;

		f.read_to_end(&mut buffer)?;

		run_preview1(buffer.as_slice(), &config)
	} else {
		info!("Start simple demo application in Hermit-WASM!");

		#[cfg(not(feature = "ci"))]
		let module_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/wasm-test.wasm"));
		#[cfg(feature = "ci")]
		let module_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/hello_world.wasm"));

		run_preview1(module_bytes, &config)
	}
}
