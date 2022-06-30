//! `tcpstream` provide an interface to establish tcp socket client.

use crate::{Handle, Handle::Socket, IpAddress, SocketHandle};

extern "Rust" {
	fn sys_tcp_stream_connect(
		ip: &[u8],
		port: u16,
		timeout: Option<u64>,
	) -> Result<SocketHandle, ()>;
	fn sys_tcp_stream_close(handle: SocketHandle) -> Result<(), ()>;
	fn sys_tcp_stream_read(handle: SocketHandle, buffer: &mut [u8]) -> Result<usize, ()>;
	fn sys_tcp_stream_write(handle: SocketHandle, buffer: &[u8]) -> Result<usize, ()>;
	fn sys_tcp_stream_set_read_timeout(
		handle: SocketHandle,
		timeout: Option<u64>,
	) -> Result<(), ()>;
	fn sys_tcp_stream_get_read_timeout(handle: SocketHandle) -> Result<Option<u64>, ()>;
	fn sys_tcp_stream_set_write_timeout(
		handle: SocketHandle,
		timeout: Option<u64>,
	) -> Result<(), ()>;
	fn sys_tcp_stream_get_write_timeout(handle: SocketHandle) -> Result<Option<u64>, ()>;
	fn sys_tcp_stream_peek(handle: SocketHandle, buf: &mut [u8]) -> Result<usize, ()>;
	fn sys_tcp_stream_set_nonblocking(handle: SocketHandle, mode: bool) -> Result<(), ()>;
	fn sys_tcp_stream_set_tll(handle: SocketHandle, ttl: u32) -> Result<(), ()>;
	fn sys_tcp_stream_get_tll(handle: SocketHandle) -> Result<u32, ()>;
	fn sys_tcp_stream_shutdown(handle: SocketHandle, how: i32) -> Result<(), ()>;
	fn sys_tcp_stream_peer_addr(handle: SocketHandle) -> Result<(IpAddress, u16), ()>;
}

/// Opens a TCP connection to a remote host.
#[inline(always)]
pub fn connect(ip: &[u8], port: u16, timeout: Option<u64>) -> Result<Handle, ()> {
	Ok(Handle::Socket(unsafe {
		sys_tcp_stream_connect(ip, port, timeout)?
	}))
}

/// Close a TCP connection
#[inline(always)]
pub fn close(handle: Handle) -> Result<(), ()> {
	match handle {
		Socket(s) => unsafe { sys_tcp_stream_close(s) },
		_ => Err(()),
	}
}

#[inline(always)]
pub fn peek(handle: Handle, buf: &mut [u8]) -> Result<usize, ()> {
	match handle {
		Socket(s) => unsafe { sys_tcp_stream_peek(s, buf) },
		_ => Err(()),
	}
}

#[inline(always)]
pub fn peer_addr(handle: Handle) -> Result<(IpAddress, u16), ()> {
	match handle {
		Socket(s) => unsafe { sys_tcp_stream_peer_addr(s) },
		_ => Err(()),
	}
}
#[inline(always)]
pub fn read(handle: Handle, buffer: &mut [u8]) -> Result<usize, ()> {
	match handle {
		Socket(s) => unsafe { sys_tcp_stream_read(s, buffer) },
		_ => Err(()),
	}
}

#[inline(always)]
pub fn write(handle: Handle, buffer: &[u8]) -> Result<usize, ()> {
	match handle {
		Socket(s) => unsafe { sys_tcp_stream_write(s, buffer) },
		_ => Err(()),
	}
}

#[inline(always)]
pub fn set_read_timeout(handle: Handle, timeout: Option<u64>) -> Result<(), ()> {
	match handle {
		Socket(s) => unsafe { sys_tcp_stream_set_read_timeout(s, timeout) },
		_ => Err(()),
	}
}

#[inline(always)]
pub fn set_write_timeout(handle: Handle, timeout: Option<u64>) -> Result<(), ()> {
	match handle {
		Socket(s) => unsafe { sys_tcp_stream_set_write_timeout(s, timeout) },
		_ => Err(()),
	}
}

#[inline(always)]
pub fn get_read_timeout(handle: Handle) -> Result<Option<u64>, ()> {
	match handle {
		Socket(s) => unsafe { sys_tcp_stream_get_read_timeout(s) },
		_ => Err(()),
	}
}

#[inline(always)]
pub fn get_write_timeout(handle: Handle) -> Result<Option<u64>, ()> {
	match handle {
		Socket(s) => unsafe { sys_tcp_stream_get_write_timeout(s) },
		_ => Err(()),
	}
}

#[inline(always)]
pub fn set_nodelay(_: Handle, mode: bool) -> Result<(), ()> {
	// smoltcp does not support Nagle's algorithm
	// => to enable Nagle's algorithm isn't possible
	if mode {
		Ok(())
	} else {
		Err(())
	}
}

#[inline(always)]
pub fn nodelay(_: Handle) -> Result<bool, ()> {
	// smoltcp does not support Nagle's algorithm
	// => return always true
	Ok(true)
}

#[inline(always)]
pub fn set_nonblocking(handle: Handle, mode: bool) -> Result<(), ()> {
	match handle {
		Socket(s) => unsafe { sys_tcp_stream_set_nonblocking(s, mode) },
		_ => Err(()),
	}
}

#[inline(always)]
pub fn set_tll(handle: Handle, ttl: u32) -> Result<(), ()> {
	match handle {
		Socket(s) => unsafe { sys_tcp_stream_set_tll(s, ttl) },
		_ => Err(()),
	}
}

#[inline(always)]
pub fn get_tll(handle: Handle) -> Result<u32, ()> {
	match handle {
		Socket(s) => unsafe { sys_tcp_stream_get_tll(s) },
		_ => Err(()),
	}
}

#[inline(always)]
pub fn shutdown(handle: Handle, how: i32) -> Result<(), ()> {
	match handle {
		Socket(s) => unsafe { sys_tcp_stream_shutdown(s, how) },
		_ => Err(()),
	}
}
