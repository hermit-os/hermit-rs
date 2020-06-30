use std::arch::x86_64::_rdtsc;
use std::collections::BTreeMap;
use std::future::Future;
use std::mem::MaybeUninit;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Mutex;
use std::task::{Context, Poll};

use std::u16;

use smoltcp::phy::Device;
#[cfg(feature = "trace")]
use smoltcp::phy::EthernetTracer;
use smoltcp::socket::{SocketHandle, SocketSet, TcpSocket, TcpSocketBuffer, TcpState};
use smoltcp::time::{Duration, Instant};
use smoltcp::wire::IpAddress;

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
	pub iface: smoltcp::iface::EthernetInterface<'static, 'static, 'static, EthernetTracer<T>>,
	#[cfg(not(feature = "trace"))]
	pub iface: smoltcp::iface::EthernetInterface<'static, 'static, 'static, T>,
	pub sockets: SocketSet<'static, 'static, 'static>,
	pub wait_for: BTreeMap<Handle, WaitFor>,
	pub timestamp: Instant,
}

impl<T> NetworkInterface<T>
where
	T: for<'a> Device<'a>,
{
	pub fn poll(&mut self) -> (std::option::Option<u64>, Vec<Handle>) {
		while self
			.iface
			.poll(&mut self.sockets, self.timestamp)
			.unwrap_or(true)
		{
			// just to make progress
		}

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

		let delay = self
			.iface
			.poll_delay(&self.sockets, self.timestamp)
			.map(|s| if s.millis() == 0 { 1 } else { s.millis() });

		(delay, vec)
	}

	pub fn poll_handle(&mut self, handle: Handle) -> Option<WaitForResult> {
		while self
			.iface
			.poll(&mut self.sockets, self.timestamp)
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
					None
				}
			}
			// a thread wants to write data
			WaitFor::Write => {
				if socket.can_send() {
					Some(WaitForResult::Ok)
				} else {
					None
				}
			}
			// a thread is waiting for acknowledge
			WaitFor::Close => match socket.state() {
				TcpState::Closed | TcpState::TimeWait => Some(WaitForResult::Ok),
				_ => None,
			},
			// a thread is waiting for an active connection
			WaitFor::IsActive => {
				if socket.is_active() {
					Some(WaitForResult::Ok)
				} else {
					None
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
			.map_err(|_| WriteFailed::InternalError)?;

		Ok(buffer.len())
	}
}

struct AsyncSocket(Handle);

impl Future for AsyncSocket {
	type Output = WaitForResult;

	fn poll(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Self::Output> {
		let mut guard = NIC.lock().unwrap();
		let nic = guard.as_mut().unwrap();

		match nic.poll_handle(self.0) {
			Some(result) => Poll::Ready(result),
			_ => Poll::Pending,
		}
	}
}

async fn socket_wait(handle: Handle) -> WaitForResult {
	AsyncSocket(handle).await
}

fn wait_for_result(handle: Handle, timeout: Option<u64>) -> WaitForResult {
	let start = std::time::Instant::now();
	let mut task = Box::pin(socket_wait(handle));

	// I can do this because I know that the AsyncSocket primitive I wrote never
	// actually accesses the context argument.)
	let v = MaybeUninit::uninit();
	let mut ctx: Context = unsafe { v.assume_init() };

	loop {
		match task.as_mut().poll(&mut ctx) {
			Poll::Ready(res) => {
				return res;
			}
			Poll::Pending => unsafe {
				if let Some(t) = timeout {
					if u128::from(t) < std::time::Instant::now().duration_since(start).as_millis() {
						return WaitForResult::Failed;
					}
				}

				sys_netwait(handle, timeout);
			},
		}
	}
}

#[no_mangle]
extern "C" fn uhyve_thread(_: usize) {
	loop {
		let mut guard = NIC.lock().unwrap();
		match guard.as_mut() {
			Some(iface) => {
				let (delay, handles) = iface.poll();
				// release lock
				drop(guard);

				unsafe {
					sys_netwait_and_wakeup(handles.as_slice(), delay);
				}
			}
			None => {
				warn!("Ethernet interface not available");
				return;
			}
		}
	}
}

pub fn network_init() -> Result<(), ()> {
	// initialize variable, which contains the next local endpoint
	let start_endpoint = ((unsafe { _rdtsc() as u64 }) % (u16::MAX as u64)) as u16;
	LOCAL_ENDPOINT.store(start_endpoint, Ordering::SeqCst);

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

	let result = wait_for_result(handle, Some(limit));
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
		match err {
			ReadFailed::CanRecvFailed => {
				*nic.wait_for
					.get_mut(&handle)
					.expect("Unable to find handle") = WaitFor::Read;
			}
			_ => {}
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
			Err(err) => {
				match err {
					ReadFailed::CanRecvFailed => {
						// wait for tx buffers and try the send operation
						if wait_for_result(handle, None) != WaitForResult::Ok {
							return Ok(0);
						}
					}
					_ => {
						return Err(());
					}
				}
			}
		}
	}
}

fn tcp_stream_try_write(handle: Handle, buffer: &[u8]) -> Result<usize, WriteFailed> {
	let mut guard = NIC.lock().map_err(|_| WriteFailed::InternalError)?;
	let nic = guard.as_mut().ok_or(WriteFailed::InternalError)?;

	let len = nic.write(handle, buffer).map_err(|err| {
		match err {
			WriteFailed::CanSendFailed => {
				*nic.wait_for
					.get_mut(&handle)
					.expect("Unable to find handle") = WaitFor::Write;
			}
			_ => {}
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
			Err(err) => {
				match err {
					WriteFailed::CanSendFailed => {
						// wait for tx buffers and try the send operation
						if wait_for_result(handle, None) != WaitForResult::Ok {
							return Err(());
						}
					}
					_ => {
						return Err(());
					}
				}
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

	wait_for_result(handle, None);

	Ok(())
}

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

	let result = wait_for_result(handle, None);
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
