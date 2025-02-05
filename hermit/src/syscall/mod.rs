mod math;
#[cfg(target_arch = "x86_64")]
#[macro_use]
pub(crate) mod x86_64;
mod allocator;

use core::ffi::{c_char, c_void};

use generic_once_cell::Lazy;
use hermit_abi as abi;
use spinning_top::RawSpinlock;

use crate::syscall;

pub(crate) enum SyscallNo {
	/// number of the system call `exit`
	Exit = 0,
	/// number of the system call `write`
	Write = 1,
	/// number of the system call `read`
	Read = 2,
	/// number of the system call `usleep`
	Usleep = 3,
	/// number of the system call `getpid`
	Getpid = 4,
	/// number of the system call `yield`
	Yield = 5,
	/// number of the system call `read_entropy`
	ReadEntropy = 6,
	/// number of the system call `get_processor_count`
	GetProcessorCount = 7,
	/// number of the system call `close`
	Close = 8,
	/// number of the system call `futex_wait`
	FutexWait = 9,
	/// number of the system call `futex_wake`
	FutexWake = 10,
	/// number of the system call `open`
	Open = 11,
	/// number of the system call `writev`
	Writev = 12,
	/// number of the system call `readv`
	Readv = 13,
}

#[thread_local]
static ERRNO: core::cell::UnsafeCell<i32> = core::cell::UnsafeCell::new(0);

/// Get the last error number from the thread local storage
#[no_mangle]
pub extern "C" fn sys_get_errno() -> i32 {
	unsafe { ERRNO.get().read() }
}

#[no_mangle]
pub extern "C" fn sys_futex_wait(
	address: *mut u32,
	expected: u32,
	timeout: *const abi::timespec,
	flags: u32,
) -> i32 {
	let result: i32 = syscall!(SyscallNo::FutexWait, address, expected, timeout, flags)
		.try_into()
		.unwrap();
	if result < 0 {
		unsafe {
			ERRNO.get().write(-result);
		}
	} else {
		unsafe {
			ERRNO.get().write(0);
		}
	}

	result
}

#[no_mangle]
pub extern "C" fn sys_futex_wake(address: *mut u32, count: i32) -> i32 {
	let result: i32 = syscall!(SyscallNo::FutexWake, address, count)
		.try_into()
		.unwrap();
	if result < 0 {
		unsafe {
			ERRNO.get().write(-result);
		}
	} else {
		unsafe {
			ERRNO.get().write(0);
		}
	}

	result
}

static MYPID: Lazy<RawSpinlock, u32> =
	Lazy::new(|| syscall!(SyscallNo::Getpid).try_into().unwrap());

#[no_mangle]
pub extern "C" fn sys_getpid() -> u32 {
	*MYPID
}

#[no_mangle]
pub extern "C" fn sys_exit(arg: i32) -> ! {
	syscall!(SyscallNo::Exit, arg);

	unreachable!()
}

#[no_mangle]
pub extern "C" fn sys_abort() -> ! {
	sys_exit(1)
}

#[no_mangle]
pub extern "C" fn sys_usleep(usecs: u64) {
	syscall!(SyscallNo::Usleep, usecs);
}

#[no_mangle]
pub extern "C" fn sys_spawn(
	_id: *mut abi::Tid,
	_func: extern "C" fn(usize),
	_arg: usize,
	_prio: u8,
	_core_id: isize,
) -> i32 {
	-22
}

#[no_mangle]
pub extern "C" fn sys_spawn2(
	_func: extern "C" fn(usize),
	_arg: usize,
	_prio: u8,
	_stack_size: usize,
	_core_id: isize,
) -> abi::Tid {
	0
}

#[no_mangle]
pub extern "C" fn sys_join(_id: abi::Tid) -> i32 {
	-22
}

#[no_mangle]
pub extern "C" fn sys_yield_now() {
	syscall!(SyscallNo::Yield);
}

#[no_mangle]
pub extern "C" fn sys_clock_gettime(_clock_id: u64, _tp: *mut abi::timespec) -> i32 {
	-22
}

#[no_mangle]
pub extern "C" fn sys_open(name: *const i8, flags: i32, mode: i32) -> i32 {
	let result: i32 = syscall!(SyscallNo::Open, name, flags, mode)
		.try_into()
		.unwrap();
	if result < 0 {
		unsafe {
			ERRNO.get().write(-result);
		}
	} else {
		unsafe {
			ERRNO.get().write(0);
		}
	}

	result
}

#[no_mangle]
pub extern "C" fn sys_unlink(_name: *const i8) -> i32 {
	-22
}

#[no_mangle]
pub extern "C" fn sys_rmdir(_name: *const i8) -> i32 {
	-22
}

#[no_mangle]
pub extern "C" fn sys_stat(_name: *const i8, _stat: *mut abi::stat) -> i32 {
	-22
}

#[no_mangle]
pub extern "C" fn sys_lstat(_name: *const i8, _stat: *mut abi::stat) -> i32 {
	-22
}

#[no_mangle]
pub extern "C" fn sys_fstat(_fd: i32, _stat: *mut abi::stat) -> i32 {
	-22
}

#[no_mangle]
pub extern "C" fn sys_get_processor_count() -> usize {
	syscall!(SyscallNo::GetProcessorCount).try_into().unwrap()
}

#[no_mangle]
pub extern "C" fn sys_notify(_id: usize, _count: i32) -> i32 {
	-22
}

#[no_mangle]
pub extern "C" fn sys_add_queue(_id: usize, _timeout_ns: i64) -> i32 {
	-22
}

#[no_mangle]
pub extern "C" fn sys_wait(_id: usize) -> i32 {
	-22
}

#[no_mangle]
pub extern "C" fn sys_init_queue(_id: usize) -> i32 {
	-22
}

#[no_mangle]
pub extern "C" fn sys_destroy_queue(_id: usize) -> i32 {
	-22
}

#[no_mangle]
pub extern "C" fn sys_block_current_task() {}

#[no_mangle]
pub extern "C" fn sys_block_current_task_with_timeout(_timeout: u64) {}

#[no_mangle]
pub extern "C" fn sys_wakeup_task(_tid: abi::Tid) {}

#[no_mangle]
pub extern "C" fn sys_accept(
	_s: i32,
	_addr: *mut abi::sockaddr,
	_addrlen: *mut abi::socklen_t,
) -> i32 {
	-22
}

#[no_mangle]
pub extern "C" fn sys_bind(_s: i32, _name: *const abi::sockaddr, _namelen: abi::socklen_t) -> i32 {
	-22
}

#[no_mangle]
pub extern "C" fn sys_connect(
	_s: i32,
	_name: *const abi::sockaddr,
	_namelen: abi::socklen_t,
) -> i32 {
	-22
}

#[no_mangle]
pub extern "C" fn sys_read(fd: i32, buf: *mut u8, len: usize) -> isize {
	let result: isize = syscall!(SyscallNo::Read, fd, buf, len).try_into().unwrap();

	if result < 0 {
		unsafe {
			ERRNO.get().write((-result).try_into().unwrap());
		}
	} else {
		unsafe {
			ERRNO.get().write(0);
		}
	}

	result
}

#[no_mangle]
pub extern "C" fn sys_readv(fd: i32, iov: *const u8, iovcnt: usize) -> isize {
	let result: isize = syscall!(SyscallNo::Readv, fd, iov, iovcnt)
		.try_into()
		.unwrap();

	if result < 0 {
		unsafe {
			ERRNO.get().write((-result).try_into().unwrap());
		}
	} else {
		unsafe {
			ERRNO.get().write(0);
		}
	}

	result
}

#[no_mangle]
pub extern "C" fn sys_mkdir(_name: *const i8, _mode: u32) -> i32 {
	-22
}

#[no_mangle]
pub extern "C" fn sys_read_entropy(buf: *mut u8, len: usize, flags: u32) -> isize {
	let result: isize = syscall!(SyscallNo::ReadEntropy, buf, len, flags)
		.try_into()
		.unwrap();

	if result < 0 {
		unsafe {
			ERRNO.get().write((-result).try_into().unwrap());
		}
	} else {
		unsafe {
			ERRNO.get().write(0);
		}
	}

	result
}

#[no_mangle]
pub extern "C" fn sys_recv(_socket: i32, _buf: *mut u8, _len: usize, _flags: i32) -> isize {
	-22
}

#[no_mangle]
pub extern "C" fn sys_recvfrom(
	_socket: i32,
	_buf: *mut u8,
	_len: usize,
	_flags: i32,
	_addr: *mut abi::sockaddr,
	_addrlen: *mut abi::socklen_t,
) -> isize {
	-22
}

#[no_mangle]
pub extern "C" fn sys_write(fd: i32, buf: *const u8, len: usize) -> isize {
	let result: isize = syscall!(SyscallNo::Write, fd, buf, len).try_into().unwrap();

	if result < 0 {
		unsafe {
			ERRNO.get().write((-result).try_into().unwrap());
		}
	} else {
		unsafe {
			ERRNO.get().write(0);
		}
	}

	result
}

#[no_mangle]
pub extern "C" fn sys_writev(fd: i32, iov: *const u8, iovcnt: usize) -> isize {
	let result: isize = syscall!(SyscallNo::Writev, fd, iov, iovcnt)
		.try_into()
		.unwrap();

	if result < 0 {
		unsafe {
			ERRNO.get().write((-result).try_into().unwrap());
		}
	} else {
		unsafe {
			ERRNO.get().write(0);
		}
	}

	result
}

#[no_mangle]
pub extern "C" fn sys_close(fd: i32) -> i32 {
	let result: i32 = syscall!(SyscallNo::Close, fd).try_into().unwrap();

	if result < 0 {
		unsafe {
			ERRNO.get().write(-result);
		}
	} else {
		unsafe {
			ERRNO.get().write(0);
		}
	}

	result
}

#[no_mangle]
pub extern "C" fn sys_dup(_fd: i32) -> i32 {
	-22
}

#[no_mangle]
pub extern "C" fn sys_getpeername(
	_s: i32,
	_name: *mut abi::sockaddr,
	_namelen: *mut abi::socklen_t,
) -> i32 {
	-22
}

#[no_mangle]
pub extern "C" fn sys_getsockname(
	_s: i32,
	_name: *mut abi::sockaddr,
	_namelen: *mut abi::socklen_t,
) -> i32 {
	-22
}

#[no_mangle]
pub extern "C" fn sys_getsockopt(
	_s: i32,
	_level: i32,
	_optname: i32,
	_optval: *mut c_void,
	_optlen: *mut abi::socklen_t,
) -> i32 {
	-22
}

#[no_mangle]
pub extern "C" fn sys_setsockopt(
	_s: i32,
	_level: i32,
	_optname: i32,
	_optval: *const c_void,
	_optlen: abi::socklen_t,
) -> i32 {
	-22
}

#[no_mangle]
pub extern "C" fn sys_ioctl(_s: i32, _cmd: i32, _argp: *mut c_void) -> i32 {
	-22
}

#[no_mangle]
pub extern "C" fn sys_poll(_fds: *mut abi::pollfd, _nfds: abi::nfds_t, _timeout: i32) -> i32 {
	-22
}

#[no_mangle]
pub extern "C" fn sys_listen(_s: i32, _backlog: i32) -> i32 {
	-22
}

#[no_mangle]
pub extern "C" fn sys_send(_s: i32, _mem: *const c_void, _len: usize, _flags: i32) -> isize {
	-22
}

#[no_mangle]
pub extern "C" fn sys_sendto(
	_s: i32,
	_mem: *const c_void,
	_len: usize,
	_flags: i32,
	_to: *const abi::sockaddr,
	_tolen: abi::socklen_t,
) -> isize {
	-22
}

#[no_mangle]
pub extern "C" fn sys_shutdown(_s: i32, _how: i32) -> i32 {
	-22
}

#[no_mangle]
pub extern "C" fn sys_socket(_domain: i32, _type_: i32, _protocol: i32) -> i32 {
	-22
}

#[no_mangle]
pub extern "C" fn sys_freeaddrinfo(_ai: *mut abi::addrinfo) {}

#[no_mangle]
pub extern "C" fn sys_getaddrinfo(
	_nodename: *const i8,
	_servname: *const u8,
	_res: *mut *mut abi::addrinfo,
) -> i32 {
	-22
}

#[no_mangle]
pub unsafe extern "C" fn _start(_argc: i32, _argv: *const *const c_char) -> ! {
	extern "C" {
		fn runtime_entry(argc: i32, argv: *const *const c_char, env: *const *const c_char) -> !;
	}

	let environ = core::ptr::null::<*const c_char>();
	let argv = [c"dummy".as_ptr()];

	// And finally start the application.
	runtime_entry(1, argv.as_ptr(), environ)
}
