#[cfg(feature = "smoltcp")]
pub mod uhyve;

use crossbeam_channel::{Receiver, Sender};
use std::arch::x86_64::_rdtsc;
use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Mutex;
use std::u16;

use smoltcp::phy::Device;
use smoltcp::socket::{SocketHandle, SocketSet, TcpSocket, TcpSocketBuffer, TcpState};
use smoltcp::time::Instant;
use smoltcp::wire::IpAddress;

use net::uhyve::UhyveNet;

lazy_static! {
	static ref NIC: Mutex<Option<NetworkInterface<UhyveNet>>> =
		Mutex::new(NetworkInterface::<UhyveNet>::new());
}

extern "Rust" {
	fn uhyve_netwakeup();
	fn uhyve_netwait(millis: Option<u64>);
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

#[derive(Debug, PartialEq, Eq)]
pub enum WaitFor {
	Establish,
	Read,
	Write,
	Close,
}

#[derive(Debug, PartialEq, Eq)]
pub enum WaitForResult {
	Ok,
	Failed,
}

pub struct NetworkInterface<T: for<'a> Device<'a>> {
	pub iface: smoltcp::iface::EthernetInterface<'static, 'static, 'static, T>,
	pub sockets: SocketSet<'static, 'static, 'static>,
	pub channels: BTreeMap<Handle, (WaitFor, Sender<WaitForResult>, bool)>,
	pub timestamp: Instant,
}

impl<T> NetworkInterface<T>
where
	T: for<'a> Device<'a>,
{
	pub fn poll(&mut self) -> Option<u64> {
		self.iface
			.poll(&mut self.sockets, self.timestamp)
			.map(|_| {
				trace!("receive message {}", self.counter);
				self.counter += 1;
			})
			.unwrap_or_else(|e| debug!("Poll: {:?}", e));

		// check if we have to inform a thread, which waits for input
		for (handle, (wait, tx, complete)) in self.channels.iter_mut() {
			let socket = self.sockets.get::<TcpSocket>(*handle);

			if !*complete {
				match wait {
					// a thread is trying to establish a connection
					WaitFor::Establish => match socket.state() {
						TcpState::Established => {
							if tx.try_send(WaitForResult::Ok).is_ok() {
								*complete = true;
							}
						}
						TcpState::FinWait1
						| TcpState::FinWait2
						| TcpState::Closing
						| TcpState::TimeWait
						| TcpState::LastAck
						| TcpState::Closed => {
							if tx.try_send(WaitForResult::Failed).is_ok() {
								*complete = true;
							}
						}
						_ => {}
					},
					// a thread wants to read data
					WaitFor::Read => {
						if socket.can_recv() {
							if tx.try_send(WaitForResult::Ok).is_ok() {
								*complete = true;
							}
						} else if !socket.may_recv() {
							if tx.try_send(WaitForResult::Failed).is_ok() {
								*complete = true;
							}
						}
					}
					// a thread wants to write data
					WaitFor::Write => {
						if socket.can_send() {
							if tx.try_send(WaitForResult::Ok).is_ok() {
								*complete = true;
							}
						}
					}
					// a thread is waiting for acknowledge
					WaitFor::Close => match socket.state() {
						TcpState::Closed | TcpState::TimeWait => {
							if tx.try_send(WaitForResult::Ok).is_ok() {
								*complete = true;
							}
						}
						_ => {}
					},
				}
			}
		}

		self.iface
			.poll_delay(&self.sockets, self.timestamp)
			.map(|s| if s.millis() == 0 { 1 } else { s.millis() })
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
			trace!(
				"receive {} bytes, {}",
				len,
				std::str::from_utf8(&buffer).unwrap().to_owned()
			);
			Ok(len)
		} else {
			Err(ReadFailed::CanRecvFailed)
		}
	}

	pub fn write(&mut self, handle: Handle, buffer: &[u8]) -> Result<usize, WriteFailed> {
		let mut socket = self.sockets.get::<TcpSocket>(handle);
		if !socket.may_recv() {
			return Ok(0);
		} else if socket.can_send() {
			socket
				.send_slice(buffer)
				.map_err(|_| WriteFailed::InternalError)?;
			trace!(
				"sending {} bytes, {}",
				buffer.len(),
				std::str::from_utf8(&buffer).unwrap().to_owned()
			);
		} else {
			return Err(WriteFailed::CanSendFailed);
		}

		trace!("send {}", std::str::from_utf8(&buffer).unwrap().to_owned());

		Ok(buffer.len())
	}
}

#[no_mangle]
extern "C" fn uhyve_thread(_: usize) {
	loop {
		let delay = NIC.lock().unwrap().as_mut().unwrap().poll();

		unsafe {
			uhyve_netwait(delay);
		}
	}
}

pub fn network_init() -> Result<(), ()> {
	if !uhyve::is_network_available() {
		return Err(());
	}

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

	Ok(())
}

#[no_mangle]
pub fn sys_tcp_stream_connect(ip: &[u8], port: u16, timeout: Option<u64>) -> Result<Handle, ()> {
	let (tx, rx): (Sender<WaitForResult>, Receiver<WaitForResult>) = crossbeam_channel::bounded(1);
	let limit = match timeout {
		Some(t) => t,
		_ => 5000,
	};
	let handle = {
		let mut guard = NIC.lock().map_err(|_| ())?;
		let nic = guard.as_mut().ok_or(())?;
		let handle = nic.connect(ip, port)?;
		nic.channels
			.insert(handle, (WaitFor::Establish, tx.clone(), false));

		handle
	};

	unsafe {
		uhyve_netwakeup();
	}

	let result = rx
		.recv_timeout(std::time::Duration::from_millis(limit))
		.map_err(|_| ())?;
	match result {
		WaitForResult::Ok => Ok(handle),
		_ => Err(()),
	}
}

fn tcp_stream_try_read(
	handle: Handle,
	buffer: &mut [u8],
	tx: Sender<WaitForResult>,
) -> Result<usize, ReadFailed> {
	let mut guard = NIC.lock().map_err(|_| ReadFailed::InternalError)?;
	let nic = guard.as_mut().ok_or(ReadFailed::InternalError)?;

	nic.read(handle, buffer).map_err(|err| {
		match err {
			ReadFailed::CanRecvFailed => {
				*nic.channels
					.get_mut(&handle)
					.expect("Unable to find handle") = (WaitFor::Read, tx, false);
			}
			_ => {}
		}

		err
	})
}

#[no_mangle]
pub fn sys_tcp_stream_read(handle: Handle, buffer: &mut [u8]) -> Result<usize, ()> {
	let (tx, rx): (Sender<WaitForResult>, Receiver<WaitForResult>) = crossbeam_channel::bounded(1);

	loop {
		let result = tcp_stream_try_read(handle, buffer, tx.clone());

		unsafe {
			uhyve_netwakeup();
			// switch to IP thread
			sys_yield();
		}

		match result {
			Ok(len) => {
				return Ok(len);
			}
			Err(err) => {
				match err {
					ReadFailed::CanRecvFailed => {
						// wait for tx buffers and try the send operation
						if rx.recv().map_err(|_| ())? != WaitForResult::Ok {
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

fn tcp_stream_try_write(
	handle: Handle,
	buffer: &[u8],
	tx: Sender<WaitForResult>,
) -> Result<usize, WriteFailed> {
	let mut guard = NIC.lock().map_err(|_| WriteFailed::InternalError)?;
	let nic = guard.as_mut().ok_or(WriteFailed::InternalError)?;

	nic.write(handle, buffer).map_err(|err| {
		match err {
			WriteFailed::CanSendFailed => {
				*nic.channels
					.get_mut(&handle)
					.expect("Unable to find handle") = (WaitFor::Write, tx, false);
			}
			_ => {}
		}

		err
	})
}

#[no_mangle]
pub fn sys_tcp_stream_write(handle: Handle, buffer: &[u8]) -> Result<usize, ()> {
	let (tx, rx): (Sender<WaitForResult>, Receiver<WaitForResult>) = crossbeam_channel::bounded(1);

	loop {
		let result = tcp_stream_try_write(handle, buffer, tx.clone());

		unsafe {
			uhyve_netwakeup();
			// switch to IP thread
			sys_yield();
		}

		match result {
			Ok(len) => {
				return Ok(len);
			}
			Err(err) => {
				match err {
					WriteFailed::CanSendFailed => {
						// wait for tx buffers and try the send operation
						if rx.recv().map_err(|_| ())? != WaitForResult::Ok {
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
	let (tx, rx): (Sender<WaitForResult>, Receiver<WaitForResult>) = crossbeam_channel::bounded(1);
	{
		// close connection
		let mut guard = NIC.lock().map_err(|_| ())?;
		let nic = guard.as_mut().ok_or(())?;
		nic.close(handle)?;
		*nic.channels
			.get_mut(&handle)
			.expect("Unable to find handle") = (WaitFor::Close, tx.clone(), false);
	}

	unsafe {
		uhyve_netwakeup();
		// switch to IP thread
		sys_yield();
	}

	rx.recv().map_err(|_| ())?;

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
