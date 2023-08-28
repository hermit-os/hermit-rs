// FIXME: Remove once removed from std:
// https://github.com/rust-lang/rust/pull/115309
#[no_mangle]
extern "C" fn sys_network_init() -> i32 {
	0
}
