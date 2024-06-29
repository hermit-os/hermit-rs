#![feature(thread_local)]
#![feature(duration_millis_float)]
#![allow(dependency_on_unit_never_type_fallback)]

use std::hint::black_box;
use std::time::{Instant, SystemTime};

use anyhow::Result;
#[cfg(target_os = "hermit")]
use hermit as _;
use log::{debug, info};
use wasmtime::*;

#[cfg(target_os = "hermit")]
mod arch;
#[cfg(target_os = "hermit")]
mod capi;

// Number of iteration to stress the benchmark
const N: u64 = 1000000;

#[inline(never)]
pub fn native_fibonacci(n: u64) -> u64 {
	let mut fib: u64 = 1;
	let mut fib1: u64 = 1;
	let mut fib2: u64 = 1;

	for _ in 3..=n {
		fib = fib1 + fib2;
		fib1 = fib2;
		fib2 = fib;
	}

	fib
}

fn main() -> Result<()> {
	simple_logger::init_with_level(log::Level::Info).unwrap();

	println!("Start Wasmtime demo!");

	// First step is to create the Wasm execution engine with some config.
	// In this example we are using the default configuration.
	let config = wasmtime::Config::new();
	info!("Wasmtime engine is configured as followed: {:?}", config);
	let engine = Engine::new(&config)?;

	// TODO: dirty workaround to get the WebAssembly module into
	// the VM. Find a way to inject the `.wasm` file into the VM
	// using another way
	debug!("Create Module");
	let module_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/fibonacci.wasm"));
	let now = Instant::now();
	let module = Module::new(&engine, &module_bytes[..])?;
	let elapsed = now.elapsed();
	println!("Time to create mdoule: {} msec", elapsed.as_millis());

	let imports = module.imports();
	for i in imports {
		info!("import from module `{}` symbol `{}`", i.module(), i.name());
	}

	debug!("Create Linker");
	let mut linker = Linker::new(&engine);

	// In case WASI, it is required to emulate
	// https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md

	linker.func_wrap("env", "now", || {
		match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
			Ok(n) => n.as_millis_f64(),
			Err(_) => panic!("SystemTime before UNIX EPOCH!"),
		}
	})?;
	linker.func_wrap("env", "exit", || panic!("Panic in WASM module"))?;

	// All wasm objects operate within the context of a "store". Each
	// `Store` has a type parameter to store host-specific data, which in
	// this case we're using `4` for.
	let mut store = Store::new(&engine, 4);
	let instance = linker.instantiate(&mut store, &module)?;

	debug!("Try to find function fibonacci");
	let fibonacci = instance.get_typed_func::<u64, u64>(&mut store, "fibonacci")?;

	// And finally we can call the wasm!
	debug!("Call function fibonacci");
	let result = fibonacci.call(&mut store, 30)?;
	println!("fibonacci(30) = {}", result);
	assert!(
		result == 832040,
		"Error in the calculation of fibonacci(30) "
	);

	let now = Instant::now();
	for _ in 0..N {
		let _result = black_box(fibonacci.call(&mut store, black_box(30)))?;
	}
	let elapsed = now.elapsed();
	println!(
		"Time to call {} times fibonacci(30): {} usec",
		N,
		elapsed.as_micros()
	);

	let now = Instant::now();
	for _ in 0..N {
		let _result = black_box(native_fibonacci(black_box(30)));
	}
	let elapsed = now.elapsed();
	println!(
		"Time to call {} times native_fibonacci(30): {} usec",
		N,
		elapsed.as_micros()
	);

	let bench = instance.get_typed_func::<(u64, u64), f64>(&mut store, "bench")?;
	let msec = bench.call(&mut store, (N, 30))?;
	println!("Benchmark takes {} usec", msec * 1000.0f64);

	let function_foo = instance.get_typed_func::<(), ()>(&mut store, "foo")?;
	function_foo.call(&mut store, ())?;
	let now = Instant::now();
	for _ in 0..N {
		function_foo.call(&mut store, ())?;
	}
	let elapsed = now.elapsed();
	println!("Time to call {} times foo: {} usec", N, elapsed.as_micros());

	Ok(())
}
