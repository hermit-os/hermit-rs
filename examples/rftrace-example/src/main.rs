#[cfg(target_os = "hermit")]
use hermit as _;

extern crate rftrace as _;
use rftrace_frontend as rftrace;

fn main() {
	let events = rftrace::init(2000, false);
	rftrace::enable();
	f1();
	std::hint::black_box(());
	// Uhyve mounts at `/host`, virtiofsd mounts to `/root`
	rftrace::dump_full_uftrace(events, "/root/tracedir", "rftrace-example").unwrap();
}

#[inline(never)]
fn f1() {
	f2();
	std::hint::black_box(());
}

#[inline(never)]
fn f2() {
	f3();
	std::hint::black_box(());
}

#[inline(never)]
fn f3() {
	std::hint::black_box(());
}
