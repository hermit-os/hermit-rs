#![cfg_attr(
	not(any(feature = "tcp", feature = "udp", feature = "dns")),
	allow(dead_code)
)]

use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use std::{fmt, io};

use wasmtime::component::{Resource, ResourceTableError};

use crate::preview2::WasiCtx;
use crate::preview2::bindings::sockets::instance_network;
use crate::preview2::bindings::sockets::network::{
	self, ErrorCode, IpAddressFamily, IpSocketAddress, Ipv4SocketAddress, Ipv6SocketAddress,
};

pub mod ip_name_lookup;
pub mod tcp;
pub mod udp;

pub struct Network;

/// Error of a `wasi:sockets` operation, either reported to the guest
/// as `error-code` or turned into a trap.
#[derive(Debug)]
pub enum SocketError {
	Code(ErrorCode),
	Trap(wasmtime::Error),
}

pub type SocketResult<T> = Result<T, SocketError>;

impl fmt::Display for SocketError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			SocketError::Code(code) => write!(f, "{code:?}"),
			SocketError::Trap(e) => write!(f, "{e}"),
		}
	}
}

impl std::error::Error for SocketError {}

impl From<ErrorCode> for SocketError {
	fn from(code: ErrorCode) -> Self {
		SocketError::Code(code)
	}
}

impl From<ResourceTableError> for SocketError {
	fn from(e: ResourceTableError) -> Self {
		SocketError::Trap(e.into())
	}
}

impl From<io::Error> for SocketError {
	fn from(e: io::Error) -> Self {
		use io::ErrorKind::*;

		SocketError::Code(match e.kind() {
			ConnectionRefused => ErrorCode::ConnectionRefused,
			ConnectionReset => ErrorCode::ConnectionReset,
			ConnectionAborted => ErrorCode::ConnectionAborted,
			AddrInUse => ErrorCode::AddressInUse,
			AddrNotAvailable => ErrorCode::AddressNotBindable,
			HostUnreachable | NetworkUnreachable => ErrorCode::RemoteUnreachable,
			NotConnected => ErrorCode::InvalidState,
			WouldBlock => ErrorCode::WouldBlock,
			TimedOut => ErrorCode::Timeout,
			PermissionDenied => ErrorCode::AccessDenied,
			InvalidInput => ErrorCode::InvalidArgument,
			Unsupported => ErrorCode::NotSupported,
			OutOfMemory => ErrorCode::OutOfMemory,
			_ => ErrorCode::Unknown,
		})
	}
}

fn to_std_addr(addr: IpSocketAddress) -> SocketAddr {
	match addr {
		IpSocketAddress::Ipv4(addr) => {
			let (a, b, c, d) = addr.address;
			SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(a, b, c, d), addr.port))
		}
		IpSocketAddress::Ipv6(addr) => {
			let (a, b, c, d, e, f, g, h) = addr.address;
			SocketAddr::V6(SocketAddrV6::new(
				Ipv6Addr::new(a, b, c, d, e, f, g, h),
				addr.port,
				addr.flow_info,
				addr.scope_id,
			))
		}
	}
}

fn from_std_addr(addr: SocketAddr) -> IpSocketAddress {
	match addr {
		SocketAddr::V4(addr) => {
			let [a, b, c, d] = addr.ip().octets();
			IpSocketAddress::Ipv4(Ipv4SocketAddress {
				port: addr.port(),
				address: (a, b, c, d),
			})
		}
		SocketAddr::V6(addr) => {
			let [a, b, c, d, e, f, g, h] = addr.ip().segments();
			IpSocketAddress::Ipv6(Ipv6SocketAddress {
				port: addr.port(),
				flow_info: addr.flowinfo(),
				address: (a, b, c, d, e, f, g, h),
				scope_id: addr.scope_id(),
			})
		}
	}
}

fn family_of(addr: &SocketAddr) -> IpAddressFamily {
	match addr {
		SocketAddr::V4(_) => IpAddressFamily::Ipv4,
		SocketAddr::V6(_) => IpAddressFamily::Ipv6,
	}
}
impl network::Host for WasiCtx {
	fn convert_error_code(&mut self, err: SocketError) -> wasmtime::Result<ErrorCode> {
		match err {
			SocketError::Code(code) => Ok(code),
			SocketError::Trap(e) => Err(e),
		}
	}

	fn network_error_code(
		&mut self,
		_err: Resource<wasmtime::Error>,
	) -> wasmtime::Result<Option<ErrorCode>> {
		Ok(None)
	}
}

impl network::HostNetwork for WasiCtx {
	fn drop(&mut self, network: Resource<Network>) -> wasmtime::Result<()> {
		self.table.delete(network)?;
		Ok(())
	}
}

impl instance_network::Host for WasiCtx {
	fn instance_network(&mut self) -> wasmtime::Result<Resource<Network>> {
		Ok(self.table.push(Network)?)
	}
}
