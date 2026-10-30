#![cfg_attr(not(feature = "tcp"), allow(dead_code))]

use std::io::{self, Read, Write};
use std::net::{
	Ipv4Addr, Ipv6Addr, Shutdown, SocketAddr, SocketAddrV4, SocketAddrV6, TcpListener, TcpStream,
};
use std::os::fd::AsRawFd;
use std::sync::Arc;
use std::{fmt, mem};

#[cfg(target_os = "hermit")]
use hermit_abi as libc;
use wasmtime::component::{Resource, ResourceTableError};
use wasmtime_wasi_io::bytes::Bytes;
use wasmtime_wasi_io::poll::{DynPollable, Pollable, subscribe};
use wasmtime_wasi_io::streams::{
	DynInputStream, DynOutputStream, InputStream, OutputStream, StreamError, StreamResult,
};

use crate::preview2::WasiCtx;
use crate::preview2::bindings::clocks::monotonic_clock::Duration;
use crate::preview2::bindings::sockets::network::{
	self, ErrorCode, IpAddressFamily, IpSocketAddress, Ipv4SocketAddress, Ipv6SocketAddress,
};
use crate::preview2::bindings::sockets::tcp::{self, ShutdownType};
use crate::preview2::bindings::sockets::{instance_network, tcp_create_socket};
use crate::preview2::io::FdReady;

/// Maximum number of bytes accepted by a single write to a TCP stream
const WRITE_BUFFER_SIZE: usize = 64 * 1024;

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

/// State of a TCP socket.
///
/// `std::net` does not allow creating unbound sockets, so binding is
/// performed in `finish-bind` by creating a listener and connecting in
/// `start-connect` by creating a stream.
enum TcpState {
	Default,
	BindStarted(SocketAddr),
	Bound(TcpListener),
	ListenStarted(TcpListener),
	Listening(TcpListener),
	Connecting(io::Result<TcpStream>),
	Connected(Arc<TcpStream>),
	Closed,
}

pub struct TcpSocket {
	family: IpAddressFamily,
	state: TcpState,
}

#[wasmtime_wasi_io::async_trait]
impl Pollable for TcpSocket {
	async fn ready(&mut self) {
		if let TcpState::Listening(listener) = &self.state {
			FdReady {
				fd: listener.as_raw_fd(),
				events: libc::POLLIN,
			}
			.await
		}
	}
}

struct TcpReadStream {
	stream: Arc<TcpStream>,
}

#[wasmtime_wasi_io::async_trait]
impl Pollable for TcpReadStream {
	async fn ready(&mut self) {
		FdReady {
			fd: self.stream.as_raw_fd(),
			events: libc::POLLIN,
		}
		.await
	}
}

impl InputStream for TcpReadStream {
	fn read(&mut self, size: usize) -> StreamResult<Bytes> {
		let mut buf = vec![0; size];
		match (&*self.stream).read(&mut buf) {
			Ok(0) if size > 0 => Err(StreamError::Closed),
			Ok(n) => {
				buf.truncate(n);
				Ok(buf.into())
			}
			Err(e) if e.kind() == io::ErrorKind::WouldBlock => Ok(Bytes::new()),
			Err(e) => Err(StreamError::LastOperationFailed(e.into())),
		}
	}
}

struct TcpWriteStream {
	stream: Arc<TcpStream>,
	/// Data accepted by `write`, but not yet sent
	pending: Vec<u8>,
}

impl TcpWriteStream {
	/// Sends as much pending data as possible without blocking.
	fn send_pending(&mut self) -> StreamResult<()> {
		while !self.pending.is_empty() {
			match (&*self.stream).write(&self.pending) {
				Ok(0) => return Err(StreamError::Closed),
				Ok(n) => {
					self.pending.drain(..n);
				}
				Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(()),
				Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {
					return Err(StreamError::Closed);
				}
				Err(e) => return Err(StreamError::LastOperationFailed(e.into())),
			}
		}
		Ok(())
	}
}

#[wasmtime_wasi_io::async_trait]
impl Pollable for TcpWriteStream {
	async fn ready(&mut self) {
		if !self.pending.is_empty() {
			FdReady {
				fd: self.stream.as_raw_fd(),
				events: libc::POLLOUT,
			}
			.await
		}
	}
}

impl OutputStream for TcpWriteStream {
	fn write(&mut self, bytes: Bytes) -> StreamResult<()> {
		if bytes.len() > WRITE_BUFFER_SIZE - self.pending.len() {
			return Err(StreamError::trap("write exceeds permitted size"));
		}
		self.pending.extend_from_slice(&bytes);
		self.send_pending()
	}

	fn flush(&mut self) -> StreamResult<()> {
		self.send_pending()
	}

	fn check_write(&mut self) -> StreamResult<usize> {
		self.send_pending()?;
		Ok(WRITE_BUFFER_SIZE - self.pending.len())
	}
}

impl WasiCtx {
	fn tcp_socket(&mut self, socket: &Resource<TcpSocket>) -> SocketResult<&mut TcpSocket> {
		Ok(self.table.get_mut(socket)?)
	}

	fn tcp_streams(
		&mut self,
		stream: TcpStream,
	) -> SocketResult<(
		Arc<TcpStream>,
		Resource<DynInputStream>,
		Resource<DynOutputStream>,
	)> {
		stream.set_nonblocking(true)?;
		let stream = Arc::new(stream);
		let input: DynInputStream = Box::new(TcpReadStream {
			stream: stream.clone(),
		});
		let output: DynOutputStream = Box::new(TcpWriteStream {
			stream: stream.clone(),
			pending: Vec::new(),
		});
		Ok((stream, self.table.push(input)?, self.table.push(output)?))
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

impl tcp_create_socket::Host for WasiCtx {
	fn create_tcp_socket(
		&mut self,
		address_family: IpAddressFamily,
	) -> SocketResult<Resource<TcpSocket>> {
		let socket = TcpSocket {
			family: address_family,
			state: TcpState::Default,
		};
		Ok(self.table.push(socket)?)
	}
}

impl tcp::Host for WasiCtx {}

impl tcp::HostTcpSocket for WasiCtx {
	fn start_bind(
		&mut self,
		socket: Resource<TcpSocket>,
		_network: Resource<Network>,
		local_address: IpSocketAddress,
	) -> SocketResult<()> {
		let addr = to_std_addr(local_address);
		let socket = self.tcp_socket(&socket)?;
		if family_of(&addr) != socket.family {
			return Err(ErrorCode::InvalidArgument.into());
		}
		if !matches!(socket.state, TcpState::Default) {
			return Err(ErrorCode::InvalidState.into());
		}
		socket.state = TcpState::BindStarted(addr);
		Ok(())
	}

	fn finish_bind(&mut self, socket: Resource<TcpSocket>) -> SocketResult<()> {
		let socket = self.tcp_socket(&socket)?;
		let TcpState::BindStarted(addr) = socket.state else {
			return Err(ErrorCode::NotInProgress.into());
		};
		socket.state = TcpState::Default;
		let listener = TcpListener::bind(addr)?;
		listener.set_nonblocking(true)?;
		socket.state = TcpState::Bound(listener);
		Ok(())
	}

	fn start_connect(
		&mut self,
		socket: Resource<TcpSocket>,
		_network: Resource<Network>,
		remote_address: IpSocketAddress,
	) -> SocketResult<()> {
		let addr = to_std_addr(remote_address);
		let socket = self.tcp_socket(&socket)?;
		if family_of(&addr) != socket.family || addr.port() == 0 || addr.ip().is_unspecified() {
			return Err(ErrorCode::InvalidArgument.into());
		}
		match socket.state {
			TcpState::Default => {}
			TcpState::Bound(_) => return Err(ErrorCode::NotSupported.into()),
			_ => return Err(ErrorCode::InvalidState.into()),
		}
		socket.state = TcpState::Connecting(TcpStream::connect(addr));
		Ok(())
	}

	fn finish_connect(
		&mut self,
		socket: Resource<TcpSocket>,
	) -> SocketResult<(Resource<DynInputStream>, Resource<DynOutputStream>)> {
		let tcp_socket = self.tcp_socket(&socket)?;
		let result = match mem::replace(&mut tcp_socket.state, TcpState::Closed) {
			TcpState::Connecting(result) => result,
			state => {
				tcp_socket.state = state;
				return Err(ErrorCode::NotInProgress.into());
			}
		};

		let (stream, input, output) = self.tcp_streams(result?)?;
		self.tcp_socket(&socket)?.state = TcpState::Connected(stream);
		Ok((input, output))
	}

	fn start_listen(&mut self, socket: Resource<TcpSocket>) -> SocketResult<()> {
		let socket = self.tcp_socket(&socket)?;
		match mem::replace(&mut socket.state, TcpState::Closed) {
			TcpState::Bound(listener) => {
				socket.state = TcpState::ListenStarted(listener);
				Ok(())
			}
			state => {
				socket.state = state;
				Err(ErrorCode::InvalidState.into())
			}
		}
	}

	fn finish_listen(&mut self, socket: Resource<TcpSocket>) -> SocketResult<()> {
		let socket = self.tcp_socket(&socket)?;
		match mem::replace(&mut socket.state, TcpState::Closed) {
			TcpState::ListenStarted(listener) => {
				socket.state = TcpState::Listening(listener);
				Ok(())
			}
			state => {
				socket.state = state;
				Err(ErrorCode::NotInProgress.into())
			}
		}
	}

	fn accept(
		&mut self,
		socket: Resource<TcpSocket>,
	) -> SocketResult<(
		Resource<TcpSocket>,
		Resource<DynInputStream>,
		Resource<DynOutputStream>,
	)> {
		let socket = self.tcp_socket(&socket)?;
		let TcpState::Listening(listener) = &socket.state else {
			return Err(ErrorCode::InvalidState.into());
		};
		let family = socket.family;
		let (stream, _) = listener.accept()?;

		let (stream, input, output) = self.tcp_streams(stream)?;
		let client = self.table.push(TcpSocket {
			family,
			state: TcpState::Connected(stream),
		})?;
		Ok((client, input, output))
	}

	fn local_address(&mut self, socket: Resource<TcpSocket>) -> SocketResult<IpSocketAddress> {
		let addr = match &self.tcp_socket(&socket)?.state {
			TcpState::Bound(listener)
			| TcpState::ListenStarted(listener)
			| TcpState::Listening(listener) => listener.local_addr()?,
			TcpState::Connected(stream) => stream.local_addr()?,
			_ => return Err(ErrorCode::InvalidState.into()),
		};
		Ok(from_std_addr(addr))
	}

	fn remote_address(&mut self, socket: Resource<TcpSocket>) -> SocketResult<IpSocketAddress> {
		match &self.tcp_socket(&socket)?.state {
			TcpState::Connected(stream) => Ok(from_std_addr(stream.peer_addr()?)),
			_ => Err(ErrorCode::InvalidState.into()),
		}
	}

	fn is_listening(&mut self, socket: Resource<TcpSocket>) -> wasmtime::Result<bool> {
		Ok(matches!(
			self.table.get(&socket)?.state,
			TcpState::Listening(_)
		))
	}

	fn address_family(&mut self, socket: Resource<TcpSocket>) -> wasmtime::Result<IpAddressFamily> {
		Ok(self.table.get(&socket)?.family)
	}

	fn set_listen_backlog_size(
		&mut self,
		socket: Resource<TcpSocket>,
		value: u64,
	) -> SocketResult<()> {
		if value == 0 {
			return Err(ErrorCode::InvalidArgument.into());
		}
		match self.tcp_socket(&socket)?.state {
			TcpState::Connecting(_) | TcpState::Connected(_) => Err(ErrorCode::InvalidState.into()),
			_ => Ok(()),
		}
	}

	fn keep_alive_enabled(&mut self, _socket: Resource<TcpSocket>) -> SocketResult<bool> {
		Err(ErrorCode::NotSupported.into())
	}

	fn set_keep_alive_enabled(
		&mut self,
		_socket: Resource<TcpSocket>,
		_value: bool,
	) -> SocketResult<()> {
		Err(ErrorCode::NotSupported.into())
	}

	fn keep_alive_idle_time(&mut self, _socket: Resource<TcpSocket>) -> SocketResult<Duration> {
		Err(ErrorCode::NotSupported.into())
	}

	fn set_keep_alive_idle_time(
		&mut self,
		_socket: Resource<TcpSocket>,
		_value: Duration,
	) -> SocketResult<()> {
		Err(ErrorCode::NotSupported.into())
	}

	fn keep_alive_interval(&mut self, _socket: Resource<TcpSocket>) -> SocketResult<Duration> {
		Err(ErrorCode::NotSupported.into())
	}

	fn set_keep_alive_interval(
		&mut self,
		_socket: Resource<TcpSocket>,
		_value: Duration,
	) -> SocketResult<()> {
		Err(ErrorCode::NotSupported.into())
	}

	fn keep_alive_count(&mut self, _socket: Resource<TcpSocket>) -> SocketResult<u32> {
		Err(ErrorCode::NotSupported.into())
	}

	fn set_keep_alive_count(
		&mut self,
		_socket: Resource<TcpSocket>,
		_value: u32,
	) -> SocketResult<()> {
		Err(ErrorCode::NotSupported.into())
	}

	fn hop_limit(&mut self, _socket: Resource<TcpSocket>) -> SocketResult<u8> {
		Err(ErrorCode::NotSupported.into())
	}

	fn set_hop_limit(&mut self, _socket: Resource<TcpSocket>, _value: u8) -> SocketResult<()> {
		Err(ErrorCode::NotSupported.into())
	}

	fn receive_buffer_size(&mut self, _socket: Resource<TcpSocket>) -> SocketResult<u64> {
		Err(ErrorCode::NotSupported.into())
	}

	fn set_receive_buffer_size(
		&mut self,
		_socket: Resource<TcpSocket>,
		_value: u64,
	) -> SocketResult<()> {
		Err(ErrorCode::NotSupported.into())
	}

	fn send_buffer_size(&mut self, _socket: Resource<TcpSocket>) -> SocketResult<u64> {
		Err(ErrorCode::NotSupported.into())
	}

	fn set_send_buffer_size(
		&mut self,
		_socket: Resource<TcpSocket>,
		_value: u64,
	) -> SocketResult<()> {
		Err(ErrorCode::NotSupported.into())
	}

	fn subscribe(
		&mut self,
		socket: Resource<TcpSocket>,
	) -> wasmtime::Result<Resource<DynPollable>> {
		subscribe(&mut self.table, socket)
	}

	fn shutdown(
		&mut self,
		socket: Resource<TcpSocket>,
		shutdown_type: ShutdownType,
	) -> SocketResult<()> {
		let TcpState::Connected(stream) = &self.tcp_socket(&socket)?.state else {
			return Err(ErrorCode::InvalidState.into());
		};
		let how = match shutdown_type {
			ShutdownType::Receive => Shutdown::Read,
			ShutdownType::Send => Shutdown::Write,
			ShutdownType::Both => Shutdown::Both,
		};
		stream.shutdown(how)?;
		Ok(())
	}

	fn drop(&mut self, socket: Resource<TcpSocket>) -> wasmtime::Result<()> {
		self.table.delete(socket)?;
		Ok(())
	}
}
