#![cfg_attr(target_os = "hermit", feature(thread_local))]

#[cfg(target_os = "hermit")]
use hermit as _;

#[cfg(target_os = "hermit")]
mod arch;
#[cfg(target_os = "hermit")]
mod capi;
pub mod preview1;
pub mod preview2;

/// Returns `true` if `bytes` contain a WebAssembly component instead of a core module.
pub fn is_component(bytes: &[u8]) -> bool {
	bytes.starts_with(b"\0asm") && bytes.get(4..8) == Some(&[0x0d, 0x00, 0x01, 0x00])
}
