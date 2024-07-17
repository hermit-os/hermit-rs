#![feature(thread_local)]
#![feature(maybe_uninit_slice)]
#![feature(duration_millis_float)]
#![allow(dependency_on_unit_never_type_fallback)]

use std::time::Instant;

use anyhow::{Context, Result};
#[cfg(target_os = "hermit")]
use hermit as _;
use log::{debug, info};
use wasmtime::*;

#[cfg(target_os = "hermit")]
mod arch;
#[cfg(target_os = "hermit")]
mod capi;
#[cfg(target_os = "hermit")]
mod preview1;

pub fn main() -> Result<()> {
	simple_logger::init_with_level(log::Level::Info)?;

	info!("Start Wasmtime demo!");

	// First step is to create the Wasm execution engine with some config.
	// In this example we are using the default configuration.
	let mut config = wasmtime::Config::new();
	config.wasm_threads(true);
	debug!("Wasmtime engine is configured as followed: {:?}", config);
	let engine = Engine::new(&config)?;

	// TODO: dirty workaround to get the WebAssembly module into
	// the VM. Find a way to inject the `.wasm` file into the VM
	// using another way
	debug!("Create Module");
	let module_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/wasm-test.wasm"));
	let now = Instant::now();
	let module = Module::new(&engine, &module_bytes[..])?;
	let elapsed = now.elapsed();
	println!("Time to create module: {} msec", elapsed.as_millis());

	debug!("Create Linker");
	#[allow(unused_mut)]
	let mut linker = Linker::new(&engine);

	#[cfg(target_os = "hermit")]
	{
		let mut imports = module.imports();
		if imports.any(|i| i.module() == "wasi_snapshot_preview1") {
			preview1::init(&mut linker)?;
		}
	}

	// All wasm objects operate within the context of a "store". Each
	// `Store` has a type parameter to store host-specific data, which in
	// this case we're using `4` for.
	let mut store = Store::new(&engine, 4);
	let instance = linker.instantiate(&mut store, &module)?;

	debug!("Try to find symbol _start");
	let func = instance.get_func(&mut store, "_start").unwrap();

	let ty = func.ty(&store);
	if ty.params().len() > 0 {
		panic!("Currently, _start should not receive arguments");
	}

	// Invoke the function and then afterwards print all the results that came
	// out, if there are any.
	let mut results = vec![Val::null_func_ref(); ty.results().len()];
	let values = Vec::new();
	let invoke_res = func
		.call(&mut store, &values, &mut results)
		.with_context(|| "failed to invoke command default".to_string());

	info!("Return value of entry point: {:?}", invoke_res);

	Ok(())
}
