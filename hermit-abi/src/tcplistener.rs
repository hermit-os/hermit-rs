//! `tcplistener` provide an interface to establish tcp socket server.

use crate::{Handle, Handle::Socket, IpAddress, SocketHandle};

extern "Rust" {
	fn sys_tcp_listener_accept(port: u16) -> Result<(SocketHandle, IpAddress, u16), ()>;
}

/// Wait for connection at specified address.
#[inline(always)]
pub fn accept(port: u16) -> Result<(Handle, IpAddress, u16), ()> {
	let (socket, ip, port) = unsafe { sys_tcp_listener_accept(port)? };
	Ok((Handle::Socket(socket), ip, port))
}
