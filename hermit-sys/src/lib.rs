#![allow(clippy::mut_mutex_lock)]
#![allow(clippy::large_enum_variant)]
#![allow(clippy::new_ret_no_self)]

#[cfg(target_arch = "aarch64")]
extern crate aarch64;
#[macro_use]
extern crate log;
#[cfg(feature = "smoltcp")]
extern crate smoltcp;
#[cfg(target_arch = "x86_64")]
extern crate x86;
#[macro_use]
extern crate lazy_static;
extern crate libm;

mod cmath;
#[cfg(not(feature = "smoltcp"))]
mod dummy;
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
pub extern "C" fn sys_network_init() -> i32 {
	set_logger(&SysLogger).expect("Can't initialize logger");
	// Determines LevelFilter at compile time
	let log_level: Option<&'static str> = option_env!("HERMIT_LOG_LEVEL_FILTER");
	let max_level: LevelFilter = match log_level {
		Some("Error") => LevelFilter::Error,
		Some("Debug") => LevelFilter::Debug,
		Some("Off") => LevelFilter::Off,
		Some("Trace") => LevelFilter::Trace,
		Some("Warn") => LevelFilter::Warn,
		Some("Info") => LevelFilter::Info,
		_ => LevelFilter::Info,
	};
	set_max_level(max_level);

	#[cfg(feature = "smoltcp")]
	let ret: i32 = if net::network_init().is_ok() { 0 } else { -1 };
	#[cfg(not(feature = "smoltcp"))]
	let ret: i32 = -1;

	if ret < 0 {
		debug!("uhyve network isn't available!");
	}

	ret
}
