use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::time::Instant;

use chrono::Local;
use clap::Parser;
use env_logger::Builder;
#[cfg(target_os = "hermit")]
use hermit as _;
use hermit_wasm::{is_component, preview1, preview2};
use log::{LevelFilter, debug, info};
use wasmtime::component::Component;
use wasmtime::error::Context;
use wasmtime::{Engine, Module, Precompiled, Result};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(name = "hermit-wasm")]
#[command(next_line_help = true)]
pub struct Config {
	/// Compiles the WebAssembly module ahead of time, writes the result to
	/// OUTPUT and exits.
	#[arg(long, value_name = "OUTPUT")]
	pub compile: Option<PathBuf>,

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

	let config = wasmtime::Config::new();
	debug!("Configuration of Wasmtime: {config:?}");
	let engine = Engine::new(&config)?;

	if CONFIG.module_and_args.is_empty() {
		eprintln!("No WebAssembly module specified. Please provide a .wasm file to run.");
		std::process::exit(1);
	}
	let fname = &CONFIG.module_and_args[0];
	let buffer = std::fs::read(fname).with_context(|| format!("unable to read {fname:?}"))?;

	if let Some(output) = &CONFIG.compile {
		info!("Compile {fname:?} to {output:?}");
		let compiled = if is_component(&buffer) {
			engine.precompile_component(&buffer)?
		} else {
			engine.precompile_module(&buffer)?
		};
		std::fs::write(output, compiled).with_context(|| format!("unable to write {output:?}"))?;
		std::process::exit(0);
	}

	info!("Start Hermit-WASM: {fname:?}");
	let now = Instant::now();
	let precompiled = Engine::detect_precompiled(&buffer);
	let code = match precompiled {
		// SAFETY: precompiled files contain native code, which is executed
		// without validation, so only files from trusted sources may be loaded.
		Some(Precompiled::Component) => {
			let component = unsafe { Component::deserialize(&engine, &buffer)? };
			debug!("Time to load component: {} msec", now.elapsed().as_millis());
			preview2::run(&engine, &component, &CONFIG.module_and_args)?
		}
		Some(Precompiled::Module) => {
			let module = unsafe { Module::deserialize(&engine, &buffer)? };
			debug!("Time to load module: {} msec", now.elapsed().as_millis());
			preview1::run(&engine, &module, &CONFIG.module_and_args)?
		}
		None if is_component(&buffer) => {
			let component = Component::new(&engine, &buffer)?;
			debug!(
				"Time to create component: {} msec",
				now.elapsed().as_millis()
			);
			preview2::run(&engine, &component, &CONFIG.module_and_args)?
		}
		None => {
			let module = Module::new(&engine, &buffer)?;
			debug!("Time to create module: {} msec", now.elapsed().as_millis());
			preview1::run(&engine, &module, &CONFIG.module_and_args)?
		}
	};

	std::process::exit(code);
}
