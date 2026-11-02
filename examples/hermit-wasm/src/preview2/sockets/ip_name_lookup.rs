#![cfg_attr(not(feature = "dns"), allow(dead_code))]

use std::collections::VecDeque;
use std::net::{IpAddr, Ipv6Addr, ToSocketAddrs};

use wasmtime::component::Resource;
use wasmtime_wasi_io::poll::{DynPollable, Pollable, subscribe};

use super::{Network, SocketResult};
use crate::preview2::WasiCtx;
use crate::preview2::bindings::sockets::ip_name_lookup;
use crate::preview2::bindings::sockets::network::{ErrorCode, IpAddress};

/// Result of a name resolution.
///
/// Hermit resolves names synchronously, so the lookup is already finished
/// when `resolve-addresses` returns.
pub struct ResolveAddressStream {
	result: Result<VecDeque<IpAddress>, ErrorCode>,
}

#[wasmtime_wasi_io::async_trait]
impl Pollable for ResolveAddressStream {
	async fn ready(&mut self) {}
}

fn from_ip_addr(addr: IpAddr) -> IpAddress {
	match addr {
		IpAddr::V4(addr) => {
			let [a, b, c, d] = addr.octets();
			IpAddress::Ipv4((a, b, c, d))
		}
		IpAddr::V6(addr) => {
			let [a, b, c, d, e, f, g, h] = addr.segments();
			IpAddress::Ipv6((a, b, c, d, e, f, g, h))
		}
	}
}

fn parse_ip(name: &str) -> Option<IpAddr> {
	if let Some(name) = name
		.strip_prefix('[')
		.and_then(|name| name.strip_suffix(']'))
	{
		return name.parse::<Ipv6Addr>().ok().map(IpAddr::V6);
	}
	name.parse().ok()
}

/// Checks the syntax of an ASCII domain name.
///
/// IDNA encoding is not supported, so non-ASCII names are rejected.
fn is_valid_domain(name: &str) -> bool {
	let name = name.strip_suffix('.').unwrap_or(name);
	!name.is_empty()
		&& name.len() <= 253
		&& name.split('.').all(|label| {
			!label.is_empty()
				&& label.len() <= 63
				&& !label.starts_with('-')
				&& !label.ends_with('-')
				&& label
					.bytes()
					.all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
		})
}

fn resolve(name: &str) -> Result<VecDeque<IpAddress>, ErrorCode> {
	let addrs = (name, 0)
		.to_socket_addrs()
		.map_err(|_| ErrorCode::NameUnresolvable)?;

	let mut ips: Vec<IpAddr> = Vec::new();
	for addr in addrs {
		let ip = addr.ip().to_canonical();
		if !ips.contains(&ip) {
			ips.push(ip);
		}
	}

	if ips.is_empty() {
		return Err(ErrorCode::NameUnresolvable);
	}
	Ok(ips.into_iter().map(from_ip_addr).collect())
}

impl ip_name_lookup::Host for WasiCtx {
	fn resolve_addresses(
		&mut self,
		_network: Resource<Network>,
		name: String,
	) -> SocketResult<Resource<ResolveAddressStream>> {
		let result = if let Some(ip) = parse_ip(&name) {
			Ok(VecDeque::from([from_ip_addr(ip.to_canonical())]))
		} else if is_valid_domain(&name) {
			resolve(&name)
		} else {
			return Err(ErrorCode::InvalidArgument.into());
		};

		Ok(self.table.push(ResolveAddressStream { result })?)
	}
}

impl ip_name_lookup::HostResolveAddressStream for WasiCtx {
	fn resolve_next_address(
		&mut self,
		stream: Resource<ResolveAddressStream>,
	) -> SocketResult<Option<IpAddress>> {
		match &mut self.table.get_mut(&stream)?.result {
			Ok(addrs) => Ok(addrs.pop_front()),
			Err(code) => Err((*code).into()),
		}
	}

	fn subscribe(
		&mut self,
		stream: Resource<ResolveAddressStream>,
	) -> wasmtime::Result<Resource<DynPollable>> {
		subscribe(&mut self.table, stream)
	}

	fn drop(&mut self, stream: Resource<ResolveAddressStream>) -> wasmtime::Result<()> {
		self.table.delete(stream)?;
		Ok(())
	}
}
