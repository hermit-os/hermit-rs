#![allow(dead_code)]
#![allow(unused_imports)]

#[cfg(target_os = "hermit")]
extern crate hermit_sys;
#[cfg(feature = "instrument")]
extern crate rftrace_frontend;

fn main() {
	#[cfg(feature = "instrument")]
	let events = rftrace_frontend::init(1000000, true);
	#[cfg(feature = "instrument")]
	rftrace_frontend::enable();

	println!("Hello World!");

	#[cfg(feature = "instrument")]
	rftrace_frontend::dump_full_uftrace(events, "trace", "hello_world", true)
		.expect("Saving trace failed");
}
