mod math;
#[cfg(target_arch = "x86_64")]
#[macro_use]
pub(crate) mod x86_64;
mod allocator;

use core::ffi::{c_char, c_void};

use generic_once_cell::Lazy;
use hermit_abi as abi;
use spinning_top::RawSpinlock;

pub type Pid = i32;

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
	/// number of the system call `fork`
	Fork = 14,
	/// number of the system call `waitpid`
	Waitpid = 15,
	/// number of the system call `spawn_process`
	SpawnProcess = 16,
	/// number of the system call `clock_gettime`
	ClockGettime = 17,
	/// number of the system call `spawn`
	Spawn = 18,
	/// number of the system call `spawn2`
	Spawn2 = 19,
	/// number of the system call `join`
	Join = 20,
	/// number of the system call `unlink`
	Unlink = 21,
	/// number of the system call `mkdir`
	Mkdir = 22,
	/// number of the system call `rmdir`
	Rmdir = 23,
	/// number of the system call `stat`
	Stat = 24,
	/// number of the system call `lstat`
	Lstat = 25,
	/// number of the system call `fstat`
	Fstat = 26,
	/// number of the system call `dup`
	Dup = 27,
	/// number of the system call `ioctl`
	Ioctl = 28,
	/// number of the system call `poll`
	Poll = 29,
	/// number of the system call `notify`
	Notify = 30,
	/// number of the system call `add_queue`
	AddQueue = 31,
	/// number of the system call `wait`
	Wait = 32,
	/// number of the system call `init_queue`
	InitQueue = 33,
	/// number of the system call `destroy_queue`
	DestroyQueue = 34,
	/// number of the system call `block_current_task`
	BlockCurrentTask = 35,
	/// number of the system call `block_current_task_with_timeout`
	BlockCurrentTaskWithTimeout = 36,
	/// number of the system call `wakeup_task`
	WakeupTask = 37,
	/// number of the system call `socket`
	Socket = 38,
	/// number of the system call `bind`
	Bind = 39,
	/// number of the system call `listen`
	Listen = 40,
	/// number of the system call `accept`
	Accept = 41,
	/// number of the system call `connect`
	Connect = 42,
	/// number of the system call `recv`
	Recv = 43,
	/// number of the system call `recvfrom`
	Recvfrom = 44,
	/// number of the system call `send`
	Send = 45,
	/// number of the system call `sendto`
	Sendto = 46,
	/// number of the system call `shutdown`
	Shutdown = 47,
	/// number of the system call `getpeername`
	Getpeername = 48,
	/// number of the system call `getsockname`
	Getsockname = 49,
	/// number of the system call `getsockopt`
	Getsockopt = 50,
	/// number of the system call `setsockopt`
	Setsockopt = 51,
	/// number of the system call `getaddrinfo`
	Getaddrinfo = 52,
	/// number of the system call `freeaddrinfo`
	Freeaddrinfo = 53,
}

#[thread_local]
static ERRNO: core::cell::UnsafeCell<i32> = core::cell::UnsafeCell::new(0);

macro_rules! update_errno {
	($ret:expr) => {
		let errno = -$ret.max(0);
		let errno = i32::try_from(errno).unwrap();
		// SAFETY: ERRNO is thread-local
		unsafe {
			ERRNO.get().write(errno);
		}
	};
}

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
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_futex_wake(address: *mut u32, count: i32) -> i32 {
	let result: i32 = syscall!(SyscallNo::FutexWake, address, count)
		.try_into()
		.unwrap();
	update_errno!(result);
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
	id: *mut abi::Tid,
	func: extern "C" fn(usize),
	arg: usize,
	prio: u8,
	core_id: isize,
) -> i32 {
	let result: i32 = syscall!(SyscallNo::Spawn, id, func, arg, prio, core_id)
		.try_into()
		.unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_spawn2(
	func: extern "C" fn(usize),
	arg: usize,
	prio: u8,
	stack_size: usize,
	core_id: isize,
) -> abi::Tid {
	syscall!(SyscallNo::Spawn2, func, arg, prio, stack_size, core_id)
		.try_into()
		.unwrap()
}

#[no_mangle]
pub extern "C" fn sys_join(id: abi::Tid) -> i32 {
	let result: i32 = syscall!(SyscallNo::Join, id).try_into().unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_yield() {
	syscall!(SyscallNo::Yield);
}

#[no_mangle]
pub extern "C" fn sys_clock_gettime(clock_id: u64, tp: *mut abi::timespec) -> i32 {
	syscall!(SyscallNo::ClockGettime, clock_id, tp) as i32
}

#[no_mangle]
pub extern "C" fn sys_open(name: *const i8, flags: i32, mode: i32) -> i32 {
	let result: i32 = syscall!(SyscallNo::Open, name, flags, mode)
		.try_into()
		.unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_unlink(name: *const i8) -> i32 {
	let result: i32 = syscall!(SyscallNo::Unlink, name).try_into().unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_rmdir(name: *const i8) -> i32 {
	let result: i32 = syscall!(SyscallNo::Rmdir, name).try_into().unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_stat(name: *const i8, stat: *mut abi::stat) -> i32 {
	let result: i32 = syscall!(SyscallNo::Stat, name, stat).try_into().unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_lstat(name: *const i8, stat: *mut abi::stat) -> i32 {
	let result: i32 = syscall!(SyscallNo::Lstat, name, stat).try_into().unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_fstat(fd: i32, stat: *mut abi::stat) -> i32 {
	let result: i32 = syscall!(SyscallNo::Fstat, fd, stat).try_into().unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_get_processor_count() -> usize {
	syscall!(SyscallNo::GetProcessorCount).try_into().unwrap()
}

#[no_mangle]
pub extern "C" fn sys_notify(id: usize, count: i32) -> i32 {
	let result: i32 = syscall!(SyscallNo::Notify, id, count).try_into().unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_add_queue(id: usize, timeout_ns: i64) -> i32 {
	let result: i32 = syscall!(SyscallNo::AddQueue, id, timeout_ns)
		.try_into()
		.unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_wait(id: usize) -> i32 {
	let result: i32 = syscall!(SyscallNo::Wait, id).try_into().unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_init_queue(id: usize) -> i32 {
	let result: i32 = syscall!(SyscallNo::InitQueue, id).try_into().unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_destroy_queue(id: usize) -> i32 {
	let result: i32 = syscall!(SyscallNo::DestroyQueue, id).try_into().unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_block_current_task() {
	syscall!(SyscallNo::BlockCurrentTask);
}

#[no_mangle]
pub extern "C" fn sys_block_current_task_with_timeout(timeout: u64) {
	syscall!(SyscallNo::BlockCurrentTaskWithTimeout, timeout);
}

#[no_mangle]
pub extern "C" fn sys_wakeup_task(tid: abi::Tid) {
	syscall!(SyscallNo::WakeupTask, tid);
}

#[no_mangle]
pub extern "C" fn sys_accept(
	s: i32,
	addr: *mut abi::sockaddr,
	addrlen: *mut abi::socklen_t,
) -> i32 {
	let result: i32 = syscall!(SyscallNo::Accept, s, addr, addrlen)
		.try_into()
		.unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_bind(s: i32, name: *const abi::sockaddr, namelen: abi::socklen_t) -> i32 {
	let result: i32 = syscall!(SyscallNo::Bind, s, name, namelen)
		.try_into()
		.unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_connect(s: i32, name: *const abi::sockaddr, namelen: abi::socklen_t) -> i32 {
	let result: i32 = syscall!(SyscallNo::Connect, s, name, namelen)
		.try_into()
		.unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_read(fd: i32, buf: *mut u8, len: usize) -> isize {
	let result: isize = syscall!(SyscallNo::Read, fd, buf, len).try_into().unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_readv(fd: i32, iov: *const u8, iovcnt: usize) -> isize {
	let result: isize = syscall!(SyscallNo::Readv, fd, iov, iovcnt)
		.try_into()
		.unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_mkdir(name: *const i8, mode: u32) -> i32 {
	let result: i32 = syscall!(SyscallNo::Mkdir, name, mode).try_into().unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_read_entropy(buf: *mut u8, len: usize, flags: u32) -> isize {
	let result: isize = syscall!(SyscallNo::ReadEntropy, buf, len, flags)
		.try_into()
		.unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_recv(socket: i32, buf: *mut u8, len: usize, flags: i32) -> isize {
	let result: isize = syscall!(SyscallNo::Recv, socket, buf, len, flags)
		.try_into()
		.unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_recvfrom(
	socket: i32,
	buf: *mut u8,
	len: usize,
	flags: i32,
	addr: *mut abi::sockaddr,
	addrlen: *mut abi::socklen_t,
) -> isize {
	let result: isize = syscall!(SyscallNo::Recvfrom, socket, buf, len, flags, addr, addrlen)
		.try_into()
		.unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_write(fd: i32, buf: *const u8, len: usize) -> isize {
	let result: isize = syscall!(SyscallNo::Write, fd, buf, len).try_into().unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_writev(fd: i32, iov: *const u8, iovcnt: usize) -> isize {
	let result: isize = syscall!(SyscallNo::Writev, fd, iov, iovcnt)
		.try_into()
		.unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_close(fd: i32) -> i32 {
	let result: i32 = syscall!(SyscallNo::Close, fd).try_into().unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_dup(fd: i32) -> i32 {
	let result: i32 = syscall!(SyscallNo::Dup, fd).try_into().unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_getpeername(
	s: i32,
	name: *mut abi::sockaddr,
	namelen: *mut abi::socklen_t,
) -> i32 {
	let result: i32 = syscall!(SyscallNo::Getpeername, s, name, namelen)
		.try_into()
		.unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_getsockname(
	s: i32,
	name: *mut abi::sockaddr,
	namelen: *mut abi::socklen_t,
) -> i32 {
	let result: i32 = syscall!(SyscallNo::Getsockname, s, name, namelen)
		.try_into()
		.unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_getsockopt(
	s: i32,
	level: i32,
	optname: i32,
	optval: *mut c_void,
	optlen: *mut abi::socklen_t,
) -> i32 {
	let result: i32 = syscall!(SyscallNo::Getsockopt, s, level, optname, optval, optlen)
		.try_into()
		.unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_setsockopt(
	s: i32,
	level: i32,
	optname: i32,
	optval: *const c_void,
	optlen: abi::socklen_t,
) -> i32 {
	let result: i32 = syscall!(SyscallNo::Setsockopt, s, level, optname, optval, optlen)
		.try_into()
		.unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_ioctl(s: i32, cmd: i32, argp: *mut c_void) -> i32 {
	let result: i32 = syscall!(SyscallNo::Ioctl, s, cmd, argp).try_into().unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_poll(fds: *mut abi::pollfd, nfds: abi::nfds_t, timeout: i32) -> i32 {
	let result: i32 = syscall!(SyscallNo::Poll, fds, nfds, timeout)
		.try_into()
		.unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_listen(s: i32, backlog: i32) -> i32 {
	let result: i32 = syscall!(SyscallNo::Listen, s, backlog).try_into().unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_send(s: i32, mem: *const c_void, len: usize, flags: i32) -> isize {
	let result: isize = syscall!(SyscallNo::Send, s, mem, len, flags)
		.try_into()
		.unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_sendto(
	s: i32,
	mem: *const c_void,
	len: usize,
	flags: i32,
	to: *const abi::sockaddr,
	tolen: abi::socklen_t,
) -> isize {
	let result: isize = syscall!(SyscallNo::Sendto, s, mem, len, flags, to, tolen)
		.try_into()
		.unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_shutdown(s: i32, how: i32) -> i32 {
	let result: i32 = syscall!(SyscallNo::Shutdown, s, how).try_into().unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_socket(domain: i32, type_: i32, protocol: i32) -> i32 {
	let result: i32 = syscall!(SyscallNo::Socket, domain, type_, protocol)
		.try_into()
		.unwrap();
	update_errno!(result);
	result
}

#[no_mangle]
pub extern "C" fn sys_freeaddrinfo(ai: *mut abi::addrinfo) {
	syscall!(SyscallNo::Freeaddrinfo, ai);
}

#[no_mangle]
pub extern "C" fn sys_getaddrinfo(
	nodename: *const i8,
	servname: *const i8,
	hints: *const abi::addrinfo,
	res: *mut *mut abi::addrinfo,
) -> i32 {
	let result: i32 = syscall!(SyscallNo::Getaddrinfo, nodename, servname, hints, res)
		.try_into()
		.unwrap();
	update_errno!(result);
	result
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

#[no_mangle]
pub unsafe extern "C" fn sys_waitpid(pid: Pid) -> i32 {
	let result: i32 = syscall!(SyscallNo::Waitpid, pid).try_into().unwrap();

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
pub unsafe extern "C" fn sys_fork() -> Pid {
	let result: i32 = syscall!(SyscallNo::Fork).try_into().unwrap();

	if result < 0 {
		unsafe {
			ERRNO.get().write(-result);
		}
	} else {
		unsafe {
			ERRNO.get().write(0);
		}
	}

	result as Pid
}

#[no_mangle]
pub unsafe extern "C" fn sys_spawn_process(name: *const c_char) -> Pid {
	let result: i32 = syscall!(SyscallNo::SpawnProcess, name).try_into().unwrap();

	if result < 0 {
		unsafe {
			ERRNO.get().write(-result);
		}
	} else {
		unsafe {
			ERRNO.get().write(0);
		}
	}

	result as Pid
}
