/// Virtio socket support for Rust. Implements VsockListener and VsockStream
/// which are analogous to the `std::net::TcpListener` and `std::net::TcpStream`
/// types.
///
/// The implementation is derived from https://github.com/rust-vsock/vsock-rs
/// and adapted for HermitOS.
use std::io::{self, Read, Result, Write};
use std::mem::size_of;
use std::os::fd::AsRawFd;
use std::os::hermit::io::{FromRawFd, OwnedFd, RawFd};

use hermit_abi::{
	accept, bind, listen, sa_family_t, sockaddr, sockaddr_vm, socket, socklen_t, AF_VSOCK,
	SOCK_STREAM, VMADDR_CID_ANY,
};

pub type VsockAddr = sockaddr_vm;

#[doc(hidden)]
pub trait IsNegative {
	fn is_negative(&self) -> bool;
	fn negate(&self) -> i32;
}

macro_rules! impl_is_negative {
    ($($t:ident)*) => ($(impl IsNegative for $t {
        fn is_negative(&self) -> bool {
            *self < 0
        }

        fn negate(&self) -> i32 {
            i32::try_from(-(*self)).unwrap()
        }
    })*)
}

impl IsNegative for i32 {
	fn is_negative(&self) -> bool {
		*self < 0
	}

	fn negate(&self) -> i32 {
		-(*self)
	}
}
impl_is_negative! { i8 i16 i64 isize }

fn check<T: std::ops::Neg<Output = T> + std::cmp::PartialOrd<T> + IsNegative>(
	res: T,
) -> io::Result<T> {
	if res.is_negative() {
		let e = match res.negate() {
			hermit_abi::errno::EACCES => std::io::ErrorKind::PermissionDenied,
			hermit_abi::errno::EADDRINUSE => std::io::ErrorKind::AddrInUse,
			hermit_abi::errno::EADDRNOTAVAIL => std::io::ErrorKind::AddrNotAvailable,
			hermit_abi::errno::EAGAIN => std::io::ErrorKind::WouldBlock,
			hermit_abi::errno::ECONNABORTED => std::io::ErrorKind::ConnectionAborted,
			hermit_abi::errno::ECONNREFUSED => std::io::ErrorKind::ConnectionRefused,
			hermit_abi::errno::ECONNRESET => std::io::ErrorKind::ConnectionReset,
			hermit_abi::errno::EEXIST => std::io::ErrorKind::AlreadyExists,
			hermit_abi::errno::EINTR => std::io::ErrorKind::Interrupted,
			hermit_abi::errno::EINVAL => std::io::ErrorKind::InvalidInput,
			hermit_abi::errno::ENOENT => std::io::ErrorKind::NotFound,
			hermit_abi::errno::ENOTCONN => std::io::ErrorKind::NotConnected,
			hermit_abi::errno::EPERM => std::io::ErrorKind::PermissionDenied,
			hermit_abi::errno::EPIPE => std::io::ErrorKind::BrokenPipe,
			hermit_abi::errno::ETIMEDOUT => std::io::ErrorKind::TimedOut,
			_ => {
				println!("Unknown error number {}", res.negate());
				std::io::ErrorKind::InvalidInput
			}
		};
		Err(std::io::Error::from(e))
	} else {
		Ok(res)
	}
}

/// A virtio socket server, listening for connections.
#[derive(Debug)]
pub struct VsockListener {
	socket: OwnedFd,
}

impl VsockListener {
	/// Create a new VsockListener which is bound and listening on the socket address.
	pub fn bind(port: u32) -> io::Result<Self> {
		unsafe {
			let saddr = sockaddr_vm {
				svm_len: std::mem::size_of::<sockaddr_vm>().try_into().unwrap(),
				svm_family: AF_VSOCK.try_into().unwrap(),
				svm_cid: VMADDR_CID_ANY,
				svm_port: port,
				..Default::default()
			};
			let fd = socket(AF_VSOCK, SOCK_STREAM, 0);

			check(bind(
				fd,
				&saddr as *const _ as *const sockaddr,
				std::mem::size_of::<sockaddr_vm>().try_into().unwrap(),
			))?;

			// rust stdlib uses a 128 connection backlog
			check(listen(fd, 128))?;

			Ok(VsockListener {
				socket: OwnedFd::from_raw_fd(fd),
			})
		}
	}

	/// Accept a new incoming connection from this listener.
	pub fn accept(&self) -> io::Result<(VsockStream, VsockAddr)> {
		let mut vsock_addr_len: socklen_t = size_of::<sockaddr_vm>().try_into().unwrap();
		let mut vsock_addr = sockaddr_vm {
			svm_len: vsock_addr_len.try_into().unwrap(),
			svm_family: AF_VSOCK as sa_family_t,
			..Default::default()
		};

		let fd = unsafe {
			check(accept(
				self.socket.as_raw_fd(),
				&mut vsock_addr as *mut _ as *mut hermit_abi::sockaddr,
				&mut vsock_addr_len as *mut u32,
			))?
		};

		Ok((VsockStream::new(fd), vsock_addr))
	}
}

pub struct VsockStream {
	fd: OwnedFd,
}

impl VsockStream {
	pub fn new(fd: RawFd) -> Self {
		Self {
			fd: unsafe { FromRawFd::from_raw_fd(fd) },
		}
	}
}

impl Read for VsockStream {
	fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
		let result = unsafe {
			check(hermit_abi::read(
				self.fd.as_raw_fd(),
				buf.as_mut_ptr(),
				buf.len(),
			))?
		};
		Ok(result.try_into().unwrap())
	}
}

impl Write for VsockStream {
	fn write(&mut self, buf: &[u8]) -> Result<usize> {
		let result = unsafe {
			check(hermit_abi::write(
				self.fd.as_raw_fd(),
				buf.as_ptr(),
				buf.len(),
			))?
		};
		Ok(result.try_into().unwrap())
	}

	fn flush(&mut self) -> Result<()> {
		Ok(())
	}
}
