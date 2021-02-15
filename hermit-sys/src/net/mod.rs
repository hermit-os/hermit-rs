#[cfg(target_arch = "aarch64")]
use aarch64::regs::*;
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::_rdtsc;
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::future::Future;
use std::mem::MaybeUninit;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Mutex;
use std::task::{Context, Poll};

use std::u16;

#[cfg(feature = "dhcpv4")]
use smoltcp::dhcp::Dhcpv4Client;
use smoltcp::phy::Device;
#[cfg(feature = "trace")]
use smoltcp::phy::EthernetTracer;
use smoltcp::socket::{SocketHandle, SocketSet, TcpSocket, TcpSocketBuffer, TcpState};
use smoltcp::time::{Duration, Instant};
use smoltcp::wire::IpAddress;
#[cfg(feature = "dhcpv4")]
use smoltcp::wire::{IpCidr, Ipv4Address, Ipv4Cidr};

use crate::net::device::HermitNet;

pub mod device;

lazy_static! {
	static ref NIC: Mutex<Option<NetworkInterface<HermitNet>>> =
		Mutex::new(NetworkInterface::<HermitNet>::new());
}

extern "Rust" {
	fn sys_netwait(handle: Handle, millis: Option<u64>);
	fn sys_netwait_and_wakeup(handles: &[Handle], millis: Option<u64>);
}

extern "C" {
	fn sys_yield();
	fn sys_spawn(
		id: *mut Tid,
		func: extern "C" fn(usize),
		arg: usize,
		prio: u8,
		selector: isize,
	) -> i32;
}

pub type Handle = SocketHandle;
pub type Tid = u32;

/// Default keep alive interval in milliseconds
const DEFAULT_KEEP_ALIVE_INTERVAL: u64 = 75000;

static LOCAL_ENDPOINT: AtomicU16 = AtomicU16::new(0);

#[derive(Debug, PartialEq, Eq)]
pub enum WriteFailed {
	CanSendFailed,
	InternalError,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ReadFailed {
	CanRecvFailed,
	InternalError,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum WaitFor {
	Establish,
	IsActive,
	Read,
	Write,
	Close,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum WaitForResult {
	Ok,
	Failed,
}

pub struct NetworkInterface<T: for<'a> Device<'a>> {
	#[cfg(feature = "trace")]
	pub iface: smoltcp::iface::EthernetInterface<'static, EthernetTracer<T>>,
	#[cfg(not(feature = "trace"))]
	pub iface: smoltcp::iface::EthernetInterface<'static, T>,
	pub sockets: SocketSet<'static>,
	pub wait_for: BTreeMap<Handle, WaitFor>,
	#[cfg(feature = "dhcpv4")]
	dhcp: Dhcpv4Client,
	#[cfg(feature = "dhcpv4")]
	prev_cidr: Ipv4Cidr,
}

impl<T> NetworkInterface<T>
where
	T: for<'a> Device<'a>,
{
	pub fn poll(&mut self) -> (std::option::Option<Duration>, Vec<Handle>) {
		let timestamp = Instant::now();
		while self
			.iface
			.poll(&mut self.sockets, timestamp)
			.unwrap_or(true)
		{
			// just to make progress
		}
		#[cfg(feature = "dhcpv4")]
		let config = self
			.dhcp
			.poll(&mut self.iface, &mut self.sockets, timestamp)
			.unwrap_or_else(|e| {
				debug!("DHCP: {:?}", e);
				None
			});
		#[cfg(feature = "dhcpv4")]
		config.map(|config| {
			debug!("DHCP config: {:?}", config);
			if let Some(cidr) = config.address {
				if cidr != self.prev_cidr && !cidr.address().is_unspecified() {
					self.iface.update_ip_addrs(|addrs| {
						addrs.iter_mut().next().map(|addr| {
							*addr = IpCidr::Ipv4(cidr);
						});
					});
					self.prev_cidr = cidr;
					info!("Assigned a new IPv4 address: {}", cidr);
				}
			}

			config.router.map(|router| {
				self.iface
					.routes_mut()
					.add_default_ipv4_route(router)
					.unwrap()
			});
			self.iface.routes_mut().update(|routes_map| {
				routes_map
					.get(&IpCidr::new(Ipv4Address::UNSPECIFIED.into(), 0))
					.map(|default_route| {
						info!("Default gateway: {}", default_route.via_router);
					});
			});

			if config.dns_servers.iter().any(|s| s.is_some()) {
				info!("DNS servers:");
				for dns_server in config.dns_servers.iter().filter_map(|s| *s) {
					info!("- {}", dns_server);
				}
			}
		});

		let mut vec = Vec::new();
		let values: Vec<(Handle, WaitFor)> = self
			.wait_for
			.iter()
			.map(|(handle, wait)| (*handle, *wait))
			.collect();
		for (handle, wait) in values {
			if self.check_handle(handle, wait).is_some() {
				vec.push(handle);
			}
		}

		let delay = self.iface.poll_delay(&self.sockets, timestamp);

		(delay, vec)
	}

	pub fn poll_handle(&mut self, handle: Handle) -> Option<WaitForResult> {
		let timestamp = Instant::now();
		while self
			.iface
			.poll(&mut self.sockets, timestamp)
			.unwrap_or(true)
		{
			// just to make progress
		}

		if let Some(wait) = self.wait_for.get(&handle) {
			let wait = *wait;
			self.check_handle(handle, wait)
		} else {
			None
		}
	}

	fn check_handle(&mut self, handle: Handle, wait: WaitFor) -> Option<WaitForResult> {
		let socket = self.sockets.get::<TcpSocket>(handle);
		match wait {
			// a thread is trying to establish a connection
			WaitFor::Establish => match socket.state() {
				TcpState::Established => Some(WaitForResult::Ok),
				TcpState::FinWait1
				| TcpState::FinWait2
				| TcpState::Closing
				| TcpState::TimeWait
				| TcpState::LastAck
				| TcpState::Closed => Some(WaitForResult::Failed),
				_ => None,
			},
			// a thread wants to read data
			WaitFor::Read => {
				if socket.can_recv() {
					Some(WaitForResult::Ok)
				} else if !socket.may_recv() {
					Some(WaitForResult::Failed)
				} else {
					match socket.state() {
						TcpState::FinWait1
						| TcpState::FinWait2
						| TcpState::Closing
						| TcpState::Closed => Some(WaitForResult::Failed),
						_ => None,
					}
				}
			}
			// a thread wants to write data
			WaitFor::Write => {
				if socket.can_send() {
					Some(WaitForResult::Ok)
				} else {
					match socket.state() {
						TcpState::FinWait1
						| TcpState::FinWait2
						| TcpState::Closing
						| TcpState::Closed => Some(WaitForResult::Failed),
						_ => None,
					}
				}
			}
			// a thread is waiting for acknowledge
			WaitFor::Close => match socket.state() {
				TcpState::FinWait1
				| TcpState::FinWait2
				| TcpState::Closed
				| TcpState::Closing
				| TcpState::TimeWait => Some(WaitForResult::Ok),
				_ => None,
			},
			// a thread is waiting for an active connection
			WaitFor::IsActive => {
				if socket.is_active() {
					Some(WaitForResult::Ok)
				} else {
					match socket.state() {
						TcpState::FinWait1
						| TcpState::FinWait2
						| TcpState::Closing
						| TcpState::Closed => Some(WaitForResult::Failed),
						_ => None,
					}
				}
			}
		}
	}

	pub fn connect(&mut self, ip: &[u8], port: u16) -> Result<Handle, ()> {
		let tcp_rx_buffer = TcpSocketBuffer::new(vec![0; 65535]);
		let tcp_tx_buffer = TcpSocketBuffer::new(vec![0; 65535]);
		let tcp_socket = TcpSocket::new(tcp_rx_buffer, tcp_tx_buffer);
		let tcp_handle = self.sockets.add(tcp_socket);
		let address =
			IpAddress::from_str(std::str::from_utf8(ip).map_err(|_| ())?).map_err(|_| ())?;

		// request a connection
		let mut socket = self.sockets.get::<TcpSocket>(tcp_handle);
		socket
			.connect(
				(address, port),
				LOCAL_ENDPOINT.fetch_add(1, Ordering::SeqCst),
			)
			.map_err(|_| ())?;

		Ok(tcp_handle)
	}

	pub fn accept(&mut self, port: u16) -> Result<Handle, ()> {
		let tcp_rx_buffer = TcpSocketBuffer::new(vec![0; 65535]);
		let tcp_tx_buffer = TcpSocketBuffer::new(vec![0; 65535]);
		let tcp_socket = TcpSocket::new(tcp_rx_buffer, tcp_tx_buffer);
		let tcp_handle = self.sockets.add(tcp_socket);

		// request a connection
		let mut socket = self.sockets.get::<TcpSocket>(tcp_handle);
		socket.listen(port).map_err(|_| ())?;

		Ok(tcp_handle)
	}

	pub fn close(&mut self, handle: Handle) -> Result<(), ()> {
		let timestamp = Instant::now();
		while self
			.iface
			.poll(&mut self.sockets, timestamp)
			.unwrap_or(true)
		{
			// just to be sure that everything is sent
		}

		let mut socket = self.sockets.get::<TcpSocket>(handle);
		socket.close();

		Ok(())
	}

	pub fn read(&mut self, handle: Handle, buffer: &mut [u8]) -> Result<usize, ReadFailed> {
		let mut socket = self.sockets.get::<TcpSocket>(handle);
		if socket.can_recv() {
			let len = socket
				.recv_slice(buffer)
				.map_err(|_| ReadFailed::InternalError)?;

			Ok(len)
		} else {
			Err(ReadFailed::CanRecvFailed)
		}
	}

	pub fn write(&mut self, handle: Handle, buffer: &[u8]) -> Result<usize, WriteFailed> {
		let mut socket = self.sockets.get::<TcpSocket>(handle);
		if !socket.may_recv() {
			return Ok(0);
		} else if !socket.can_send() {
			return Err(WriteFailed::CanSendFailed);
		}

		socket
			.send_slice(buffer)
			.map_err(|_| WriteFailed::InternalError)
	}
}

struct AsyncSocket(Handle);

impl Future for AsyncSocket {
	type Output = WaitForResult;

	fn poll(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Self::Output> {
		let mut guard = NIC.lock().unwrap();
		let nic = guard.as_mut().unwrap();

		if let Some(result) = nic.poll_handle(self.0) {
			Poll::Ready(result)
		} else {
			Poll::Pending
		}
	}
}

async fn socket_wait(handle: Handle) -> WaitForResult {
	AsyncSocket(handle).await
}

fn wait_for_result(handle: Handle, timeout: Option<u64>, polling: bool) -> WaitForResult {
	let start = std::time::Instant::now();
	let mut task = Box::pin(socket_wait(handle));

	// I can do this because I know that the AsyncSocket primitive and
	// never use the context argument.
	// Fixme: This is UB
	let v = MaybeUninit::uninit();
	let mut ctx: Context = unsafe { v.assume_init() };

	loop {
		match task.as_mut().poll(&mut ctx) {
			Poll::Ready(res) => {
				return res;
			}
			Poll::Pending => {
				if let Some(t) = timeout {
					if u128::from(t) < std::time::Instant::now().duration_since(start).as_millis() {
						return WaitForResult::Failed;
					}
				}

				let new_timeout = if polling { Some(0) } else { timeout };
				unsafe {
					sys_netwait(handle, new_timeout);
				}
			}
		}
	}
}

#[no_mangle]
extern "C" fn uhyve_thread(_: usize) {
	loop {
		let mut guard = NIC.lock().unwrap();
		if let Some(iface) = guard.as_mut() {
			let (delay, handles) = iface.poll();
			// release lock
			drop(guard);

			unsafe {
				sys_netwait_and_wakeup(handles.as_slice(), delay.map(|s| s.millis()));
			}
		} else {
			warn!("Ethernet interface not available");
			return;
		}
	}
}

#[cfg(target_arch = "x86_64")]
fn start_endpoint() -> u16 {
	((unsafe { _rdtsc() as u64 }) % (u16::MAX as u64))
		.try_into()
		.unwrap()
}

#[cfg(target_arch = "aarch64")]
fn start_endpoint() -> u16 {
	(CNTPCT_EL0.get() % (u16::MAX as u64)).try_into().unwrap()
}

pub fn network_init() -> Result<(), ()> {
	// initialize variable, which contains the next local endpoint
	LOCAL_ENDPOINT.store(start_endpoint(), Ordering::SeqCst);

	// create thread, which manages the network stack
	// use a higher priority to reduce the network latency
	let mut tid: Tid = 0;
	let ret = unsafe { sys_spawn(&mut tid, uhyve_thread, 0, 3, 0) };
	if ret >= 0 {
		info!("Spawn network thread with id {}", tid);
	}

	// switch to
	unsafe {
		sys_yield();
	}

	Ok(())
}

#[no_mangle]
pub fn sys_tcp_stream_connect(ip: &[u8], port: u16, timeout: Option<u64>) -> Result<Handle, ()> {
	let limit = match timeout {
		Some(t) => t,
		_ => 5000,
	};
	let handle = {
		let mut guard = NIC.lock().map_err(|_| ())?;
		let nic = guard.as_mut().ok_or(())?;
		let handle = nic.connect(ip, port)?;
		nic.wait_for.insert(handle, WaitFor::Establish);

		handle
	};

	let result = wait_for_result(handle, Some(limit), false);
	match result {
		WaitForResult::Ok => {
			let mut guard = NIC.lock().map_err(|_| ())?;
			let nic = guard.as_mut().ok_or(())?;
			let mut socket = nic.sockets.get::<TcpSocket>(handle);
			socket.set_keep_alive(Some(Duration::from_millis(DEFAULT_KEEP_ALIVE_INTERVAL)));

			Ok(handle)
		}
		_ => Err(()),
	}
}

fn tcp_stream_try_read(handle: Handle, buffer: &mut [u8]) -> Result<usize, ReadFailed> {
	let mut guard = NIC.lock().map_err(|_| ReadFailed::InternalError)?;
	let nic = guard.as_mut().ok_or(ReadFailed::InternalError)?;

	nic.read(handle, buffer).map_err(|err| {
		if let ReadFailed::CanRecvFailed = err {
			*nic.wait_for
				.get_mut(&handle)
				.expect("Unable to find handle") = WaitFor::Read;
		}

		err
	})
}

#[no_mangle]
pub fn sys_tcp_stream_read(handle: Handle, buffer: &mut [u8]) -> Result<usize, ()> {
	loop {
		let result = tcp_stream_try_read(handle, buffer);

		match result {
			Ok(len) => {
				return Ok(len);
			}
			Err(ReadFailed::CanRecvFailed) => {
				// wait for tx buffers and try the send operation
				// ToDo: Is the != here correct? seems unintuitive to return ok if result is not okay
				//	Additionally timeout of None seems like a bad idea
				if wait_for_result(handle, None, false) != WaitForResult::Ok {
					return Ok(0);
				}
			}
			_ => {
				return Err(());
			}
		}
	}
}

fn tcp_stream_try_write(handle: Handle, buffer: &[u8]) -> Result<usize, WriteFailed> {
	let mut guard = NIC.lock().map_err(|_| WriteFailed::InternalError)?;
	let nic = guard.as_mut().ok_or(WriteFailed::InternalError)?;

	let len = nic.write(handle, buffer).map_err(|err| {
		if let WriteFailed::CanSendFailed = err {
			*nic.wait_for
				.get_mut(&handle)
				.expect("Unable to find handle") = WaitFor::Write;
		}

		err
	})?;

	Ok(len)
}

#[no_mangle]
pub fn sys_tcp_stream_write(handle: Handle, buffer: &[u8]) -> Result<usize, ()> {
	loop {
		let result = tcp_stream_try_write(handle, buffer);

		match result {
			Ok(len) => {
				return Ok(len);
			}
			Err(WriteFailed::CanSendFailed) => {
				// wait for tx buffers and try the send operation
				if wait_for_result(handle, None, true) != WaitForResult::Ok {
					return Err(());
				}
			}
			_ => {
				return Err(());
			}
		}
	}
}

#[no_mangle]
pub fn sys_tcp_stream_close(handle: Handle) -> Result<(), ()> {
	{
		// close connection
		let mut guard = NIC.lock().map_err(|_| ())?;
		let nic = guard.as_mut().ok_or(())?;
		nic.close(handle)?;
		*nic.wait_for
			.get_mut(&handle)
			.expect("Unable to find handle") = WaitFor::Close;
	}

	wait_for_result(handle, None, false);

	Ok(())
}

//ToDo: an enum, or at least constants would be better
#[no_mangle]
pub fn sys_tcp_stream_shutdown(handle: Handle, how: i32) -> Result<(), ()> {
	match how {
		0 /* Read */ => {
			debug!("Shutdown::Read is not implemented");
			Ok(())
		},
		1 /* Write */ => {
			sys_tcp_stream_close(handle)
		},
		2 /* Both */ => {
			sys_tcp_stream_close(handle)
		},
		_ => {
			panic!("Invalid shutdown argument {}", how);
		},
	}
}

#[no_mangle]
pub fn sys_tcp_stream_set_read_timeout(_handle: Handle, _timeout: Option<u64>) -> Result<(), ()> {
	Err(())
}

#[no_mangle]
pub fn sys_tcp_stream_get_read_timeout(_handle: Handle) -> Result<Option<u64>, ()> {
	Err(())
}

#[no_mangle]
pub fn sys_tcp_stream_set_write_timeout(_handle: Handle, _timeout: Option<u64>) -> Result<(), ()> {
	Err(())
}

#[no_mangle]
pub fn sys_tcp_stream_get_write_timeout(_handle: Handle) -> Result<Option<u64>, ()> {
	Err(())
}

#[deprecated(since = "0.1.14", note = "Please don't use this function")]
#[no_mangle]
pub fn sys_tcp_stream_duplicate(_handle: Handle) -> Result<Handle, ()> {
	Err(())
}

#[no_mangle]
pub fn sys_tcp_stream_peek(_handle: Handle, _buf: &mut [u8]) -> Result<usize, ()> {
	Err(())
}

#[no_mangle]
pub fn sys_tcp_stream_set_nonblocking(_handle: Handle, _mode: bool) -> Result<(), ()> {
	Err(())
}

#[no_mangle]
pub fn sys_tcp_stream_set_tll(_handle: Handle, _ttl: u32) -> Result<(), ()> {
	Err(())
}

#[no_mangle]
pub fn sys_tcp_stream_get_tll(_handle: Handle) -> Result<u32, ()> {
	Err(())
}

#[no_mangle]
pub fn sys_tcp_stream_peer_addr(handle: Handle) -> Result<(IpAddress, u16), ()> {
	let mut guard = NIC.lock().map_err(|_| ())?;
	let nic = guard.as_mut().ok_or(())?;
	let mut socket = nic.sockets.get::<TcpSocket>(handle);
	socket.set_keep_alive(Some(Duration::from_millis(DEFAULT_KEEP_ALIVE_INTERVAL)));
	let endpoint = socket.remote_endpoint();

	Ok((endpoint.addr, endpoint.port))
}

#[no_mangle]
pub fn sys_tcp_listener_accept(port: u16) -> Result<(Handle, IpAddress, u16), ()> {
	let handle = {
		let mut guard = NIC.lock().map_err(|_| ())?;
		let nic = guard.as_mut().ok_or(())?;
		let handle = nic.accept(port)?;
		nic.wait_for.insert(handle, WaitFor::IsActive);

		handle
	};

	let result = wait_for_result(handle, None, false);
	match result {
		WaitForResult::Ok => {
			let mut guard = NIC.lock().map_err(|_| ())?;
			let nic = guard.as_mut().ok_or(())?;
			let mut socket = nic.sockets.get::<TcpSocket>(handle);
			socket.set_keep_alive(Some(Duration::from_millis(DEFAULT_KEEP_ALIVE_INTERVAL)));
			let endpoint = socket.remote_endpoint();

			Ok((handle, endpoint.addr, endpoint.port))
		}
		_ => Err(()),
	}
}
