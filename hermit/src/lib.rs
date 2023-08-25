#![allow(clippy::large_enum_variant)]
#![allow(clippy::new_ret_no_self)]

#[cfg(not(feature = "tcp"))]
mod dummy;

#[no_mangle]
pub extern "C" fn sys_network_init() -> i32 {
	// nothing to do

	0
}
