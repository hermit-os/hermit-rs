#![cfg_attr(not(feature = "udp"), allow(dead_code))]

use std::io;
use std::net::{SocketAddr, UdpSocket as StdUdpSocket};
use std::os::fd::AsRawFd;
use std::sync::Arc;

#[cfg(target_os = "hermit")]
use hermit_abi as libc;
use wasmtime::component::Resource;
use wasmtime_wasi_io::poll::{DynPollable, Pollable, subscribe};

use super::{Network, SocketError, SocketResult, family_of, from_std_addr, to_std_addr};
use crate::preview2::WasiCtx;
use crate::preview2::bindings::sockets::network::{ErrorCode, IpAddressFamily, IpSocketAddress};
use crate::preview2::bindings::sockets::udp::{self, IncomingDatagram, OutgoingDatagram};
use crate::preview2::bindings::sockets::udp_create_socket;
use crate::preview2::io::FdReady;

const MAX_DATAGRAM_SIZE: usize = 65535;

/// Maximum number of datagrams permitted by a single `check-send`
const MAX_DATAGRAMS: usize = 16;

fn check_remote(addr: &SocketAddr, family: IpAddressFamily) -> SocketResult<()> {
	if family_of(addr) != family || addr.port() == 0 || addr.ip().is_unspecified() {
		return Err(ErrorCode::InvalidArgument.into());
	}
	Ok(())
}

enum UdpState {
	Default,
	BindStarted(SocketAddr),
	Bound(Arc<StdUdpSocket>),
}

/// A UDP socket.
///
/// The remote address of `stream` is not passed to the kernel, but
/// applied by the datagram streams.
pub struct UdpSocket {
	family: IpAddressFamily,
	state: UdpState,
	remote: Option<SocketAddr>,
}

#[wasmtime_wasi_io::async_trait]
impl Pollable for UdpSocket {
	async fn ready(&mut self) {}
}

pub struct IncomingDatagramStream {
	socket: Arc<StdUdpSocket>,
	remote: Option<SocketAddr>,
}

#[wasmtime_wasi_io::async_trait]
impl Pollable for IncomingDatagramStream {
	async fn ready(&mut self) {
		FdReady {
			fd: self.socket.as_raw_fd(),
			events: libc::POLLIN,
		}
		.await
	}
}

pub struct OutgoingDatagramStream {
	socket: Arc<StdUdpSocket>,
	family: IpAddressFamily,
	remote: Option<SocketAddr>,
	/// Number of datagrams permitted by the last `check-send`
	permitted: usize,
}

impl OutgoingDatagramStream {
	fn target(&self, addr: Option<IpSocketAddress>) -> SocketResult<SocketAddr> {
		let addr = match (self.remote, addr.map(to_std_addr)) {
			(Some(remote), Some(addr)) if remote != addr => {
				return Err(ErrorCode::InvalidArgument.into());
			}
			(Some(remote), _) => remote,
			(None, Some(addr)) => addr,
			(None, None) => return Err(ErrorCode::InvalidArgument.into()),
		};
		check_remote(&addr, self.family)?;
		Ok(addr)
	}

	fn send_one(&self, datagram: &OutgoingDatagram) -> SocketResult<()> {
		let addr = self.target(datagram.remote_address)?;
		if datagram.data.len() > MAX_DATAGRAM_SIZE {
			return Err(ErrorCode::DatagramTooLarge.into());
		}
		self.socket.send_to(&datagram.data, addr)?;
		Ok(())
	}
}

#[wasmtime_wasi_io::async_trait]
impl Pollable for OutgoingDatagramStream {
	async fn ready(&mut self) {
		FdReady {
			fd: self.socket.as_raw_fd(),
			events: libc::POLLOUT,
		}
		.await
	}
}

impl WasiCtx {
	fn udp_socket(&mut self, socket: &Resource<UdpSocket>) -> SocketResult<&mut UdpSocket> {
		Ok(self.table.get_mut(socket)?)
	}
}

impl udp_create_socket::Host for WasiCtx {
	fn create_udp_socket(
		&mut self,
		address_family: IpAddressFamily,
	) -> SocketResult<Resource<UdpSocket>> {
		let socket = UdpSocket {
			family: address_family,
			state: UdpState::Default,
			remote: None,
		};
		Ok(self.table.push(socket)?)
	}
}

impl udp::Host for WasiCtx {}

impl udp::HostUdpSocket for WasiCtx {
	fn start_bind(
		&mut self,
		socket: Resource<UdpSocket>,
		_network: Resource<Network>,
		local_address: IpSocketAddress,
	) -> SocketResult<()> {
		let addr = to_std_addr(local_address);
		let socket = self.udp_socket(&socket)?;
		if family_of(&addr) != socket.family {
			return Err(ErrorCode::InvalidArgument.into());
		}
		if !matches!(socket.state, UdpState::Default) {
			return Err(ErrorCode::InvalidState.into());
		}
		socket.state = UdpState::BindStarted(addr);
		Ok(())
	}

	fn finish_bind(&mut self, socket: Resource<UdpSocket>) -> SocketResult<()> {
		let socket = self.udp_socket(&socket)?;
		let UdpState::BindStarted(addr) = socket.state else {
			return Err(ErrorCode::NotInProgress.into());
		};
		socket.state = UdpState::Default;
		let udp_socket = StdUdpSocket::bind(addr)?;
		udp_socket.set_nonblocking(true)?;
		socket.state = UdpState::Bound(Arc::new(udp_socket));
		Ok(())
	}

	fn stream(
		&mut self,
		socket: Resource<UdpSocket>,
		remote_address: Option<IpSocketAddress>,
	) -> SocketResult<(
		Resource<IncomingDatagramStream>,
		Resource<OutgoingDatagramStream>,
	)> {
		let socket = self.udp_socket(&socket)?;
		let UdpState::Bound(udp_socket) = &socket.state else {
			return Err(ErrorCode::InvalidState.into());
		};
		let remote = remote_address.map(to_std_addr);
		if let Some(remote) = &remote {
			check_remote(remote, socket.family)?;
		}
		socket.remote = remote;

		let incoming = IncomingDatagramStream {
			socket: udp_socket.clone(),
			remote,
		};
		let outgoing = OutgoingDatagramStream {
			socket: udp_socket.clone(),
			family: socket.family,
			remote,
			permitted: 0,
		};
		Ok((self.table.push(incoming)?, self.table.push(outgoing)?))
	}

	fn local_address(&mut self, socket: Resource<UdpSocket>) -> SocketResult<IpSocketAddress> {
		match &self.udp_socket(&socket)?.state {
			UdpState::Bound(udp_socket) => Ok(from_std_addr(udp_socket.local_addr()?)),
			_ => Err(ErrorCode::InvalidState.into()),
		}
	}

	fn remote_address(&mut self, socket: Resource<UdpSocket>) -> SocketResult<IpSocketAddress> {
		match self.udp_socket(&socket)?.remote {
			Some(remote) => Ok(from_std_addr(remote)),
			None => Err(ErrorCode::InvalidState.into()),
		}
	}

	fn address_family(&mut self, socket: Resource<UdpSocket>) -> wasmtime::Result<IpAddressFamily> {
		Ok(self.table.get(&socket)?.family)
	}

	fn unicast_hop_limit(&mut self, _socket: Resource<UdpSocket>) -> SocketResult<u8> {
		Err(ErrorCode::NotSupported.into())
	}

	fn set_unicast_hop_limit(
		&mut self,
		_socket: Resource<UdpSocket>,
		value: u8,
	) -> SocketResult<()> {
		if value == 0 {
			return Err(ErrorCode::InvalidArgument.into());
		}
		Err(ErrorCode::NotSupported.into())
	}

	fn receive_buffer_size(&mut self, _socket: Resource<UdpSocket>) -> SocketResult<u64> {
		Err(ErrorCode::NotSupported.into())
	}

	fn set_receive_buffer_size(
		&mut self,
		_socket: Resource<UdpSocket>,
		_value: u64,
	) -> SocketResult<()> {
		Err(ErrorCode::NotSupported.into())
	}

	fn send_buffer_size(&mut self, _socket: Resource<UdpSocket>) -> SocketResult<u64> {
		Err(ErrorCode::NotSupported.into())
	}

	fn set_send_buffer_size(
		&mut self,
		_socket: Resource<UdpSocket>,
		_value: u64,
	) -> SocketResult<()> {
		Err(ErrorCode::NotSupported.into())
	}

	fn subscribe(
		&mut self,
		socket: Resource<UdpSocket>,
	) -> wasmtime::Result<Resource<DynPollable>> {
		subscribe(&mut self.table, socket)
	}

	fn drop(&mut self, socket: Resource<UdpSocket>) -> wasmtime::Result<()> {
		self.table.delete(socket)?;
		Ok(())
	}
}

impl udp::HostIncomingDatagramStream for WasiCtx {
	fn receive(
		&mut self,
		stream: Resource<IncomingDatagramStream>,
		max_results: u64,
	) -> SocketResult<Vec<IncomingDatagram>> {
		let stream = self.table.get(&stream)?;
		let max_results = usize::try_from(max_results).unwrap_or(usize::MAX);
		let mut datagrams = Vec::new();
		let mut buf = Vec::new();

		while datagrams.len() < max_results {
			buf.resize(MAX_DATAGRAM_SIZE, 0);
			match stream.socket.recv_from(&mut buf) {
				Ok((_, addr)) if stream.remote.is_some_and(|remote| remote != addr) => {}
				Ok((len, addr)) => datagrams.push(IncomingDatagram {
					data: buf[..len].to_vec(),
					remote_address: from_std_addr(addr),
				}),
				Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
				Err(e) if datagrams.is_empty() => return Err(e.into()),
				Err(_) => break,
			}
		}

		Ok(datagrams)
	}

	fn subscribe(
		&mut self,
		stream: Resource<IncomingDatagramStream>,
	) -> wasmtime::Result<Resource<DynPollable>> {
		subscribe(&mut self.table, stream)
	}

	fn drop(&mut self, stream: Resource<IncomingDatagramStream>) -> wasmtime::Result<()> {
		self.table.delete(stream)?;
		Ok(())
	}
}

impl udp::HostOutgoingDatagramStream for WasiCtx {
	fn check_send(&mut self, stream: Resource<OutgoingDatagramStream>) -> SocketResult<u64> {
		let stream = self.table.get_mut(&stream)?;
		stream.permitted = MAX_DATAGRAMS;
		Ok(stream.permitted as u64)
	}

	fn send(
		&mut self,
		stream: Resource<OutgoingDatagramStream>,
		datagrams: Vec<OutgoingDatagram>,
	) -> SocketResult<u64> {
		let stream = self.table.get_mut(&stream)?;
		if datagrams.len() > stream.permitted {
			return Err(SocketError::Trap(wasmtime::format_err!(
				"send exceeds the number of datagrams permitted by check-send"
			)));
		}
		stream.permitted -= datagrams.len();

		let mut sent = 0;
		for datagram in &datagrams {
			match stream.send_one(datagram) {
				Ok(()) => sent += 1,
				Err(SocketError::Code(ErrorCode::WouldBlock)) => break,
				Err(e) if sent == 0 => return Err(e),
				Err(_) => break,
			}
		}

		Ok(sent)
	}

	fn subscribe(
		&mut self,
		stream: Resource<OutgoingDatagramStream>,
	) -> wasmtime::Result<Resource<DynPollable>> {
		subscribe(&mut self.table, stream)
	}

	fn drop(&mut self, stream: Resource<OutgoingDatagramStream>) -> wasmtime::Result<()> {
		self.table.delete(stream)?;
		Ok(())
	}
}
