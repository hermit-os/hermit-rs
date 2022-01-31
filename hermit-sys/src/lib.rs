#![allow(clippy::large_enum_variant)]
#![allow(clippy::new_ret_no_self)]

#[macro_use]
extern crate log;

mod cmath;
#[cfg(not(feature = "tcp"))]
mod dummy;
#[cfg(feature = "tcp")]
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

	#[cfg(feature = "tcp")]
	let ret: i32 = if net::network_init().is_ok() { 0 } else { -1 };
	#[cfg(not(feature = "tcp"))]
	let ret: i32 = -1;

	if ret < 0 {
		debug!("uhyve network isn't available!");
	}

	ret
}
