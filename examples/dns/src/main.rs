use std::ffi::CString;
use std::net::{Ipv4Addr, Ipv6Addr};

#[cfg(target_os = "hermit")]
use hermit as _;
use hermit_abi::{in6_addr, in_addr};

fn main() {
	println!("Search address of rust-lang.org...");

	let c_string = std::ffi::CString::new("rust-lang.org").expect("CString::new failed");
	let ptr = c_string.into_raw();
	let mut inaddr: in_addr = Default::default();
	let result = unsafe {
		hermit_abi::getaddrbyname(
			ptr,
			&mut inaddr as *mut _ as *mut u8,
			std::mem::size_of::<in_addr>(),
		)
	};

	if result < 0 {
		panic!("getaddrsbyname returns error: {}", -result);
	}

	let addr = Ipv4Addr::from(u32::from_be(inaddr.s_addr));
	println!("IPv4 address {addr}");

	let mut inaddr: in6_addr = Default::default();
	let result = unsafe {
		hermit_abi::getaddrbyname(
			ptr,
			&mut inaddr as *mut _ as *mut u8,
			std::mem::size_of::<in6_addr>(),
		)
	};

	if result < 0 {
		panic!("getaddrsbyname returns error: {}", -result);
	}

	let addr = Ipv6Addr::from(u128::from_be_bytes(inaddr.s6_addr));
	println!("IPv6 address {addr}");

	// retake pointer to free memory
	let _ = unsafe { CString::from_raw(ptr) };
}
