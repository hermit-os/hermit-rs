#![doc = include_str!("../README.md")]
#![cfg_attr(
	all(target_os = "hermit", feature = "common-os"),
	feature(thread_local)
)]

#[cfg(all(target_os = "hermit", feature = "common-os"))]
mod syscall;
