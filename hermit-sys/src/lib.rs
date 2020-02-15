#[macro_use]
extern crate log;
#[cfg(feature = "smoltcp")]
extern crate smoltcp;
extern crate x86;
#[macro_use]
extern crate lazy_static;

#[cfg(feature = "smoltcp")]
mod net;

use log::{set_logger, set_max_level, LevelFilter, Metadata, Record};

/// Data structure to filter kernel messages
struct SysLogger;

impl log::Log for SysLogger {
	fn enabled(&self, _: &Metadata) -> bool {
		true
	}

	fn flush(&self) {
		// nothing to do
	}

	fn log(&self, record: &Record) {
		if self.enabled(record.metadata()) {
			println!("[{}] {}", record.level(), record.args());
		}
	}
}

#[no_mangle]
pub extern "C" fn sys_network_init() -> i32{
	set_logger(&SysLogger).expect("Can't initialize logger");
	set_max_level(LevelFilter::Info);

	#[cfg(feature = "smoltcp")]
	let ret = net::uhyve::network_init();
	#[cfg(not(feature = "smoltcp"))]
	let ret = -1;

	ret
}
