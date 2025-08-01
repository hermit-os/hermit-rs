#![allow(dead_code)]

// This file provide the interface of WASI preview1 to HermitOS
//
// NOTE: The current version assumes that / is the current
// working directory of the WASI application.

use std::cmp::Ordering;
use std::ffi::{OsString, c_char};
use std::mem::MaybeUninit;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::{io, thread};

use anyhow::Result;
use bitflags::bitflags;
#[cfg(target_os = "hermit")]
use hermit_abi as libc;
use log::{debug, error};
use wasi::*;
use wasmtime::{AsContext, AsContextMut, Caller, Extern};
use zerocopy::{Immutable, IntoBytes, KnownLayout};

static FD: Mutex<Vec<Descriptor>> = Mutex::new(Vec::new());

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FileStream {
	pub raw_fd: i32,
	pub path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Descriptor {
	None,
	Stdin,
	Stdout,
	Stderr,
	Directory(String),
	File(FileStream),
}

impl Descriptor {
	#[inline]
	pub fn is_none(&self) -> bool {
		*self == Self::None
	}
}

bitflags! {
	/// Options for opening files
	#[derive(Debug, Copy, Clone, Default)]
	pub(crate) struct Oflags: i32 {
		/// Create file if it does not exist.
		const OFLAGS_CREAT = 1 << 0;
		/// Fail if not a directory.
		const OFLAGS_DIRECTORY = 1 << 1;
		/// Fail if file already exists.
		const OFLAGS_EXCL = 1 << 2;
		/// Truncate file to size 0.
		const OFLAGS_TRUNC = 1 << 3;
	}
}

/// The type of the file descriptor or file is unknown or is different from any of the other types specified.
const UNKNOWN: u8 = 0;
/// The file descriptor or file refers to a block device inode.
const BLOCK_DEVICE: u8 = 1 << 0;
/// The file descriptor or file refers to a character device inode.
const CHARACTER_DEVICE: u8 = 1 << 1;
/// The file descriptor or file refers to a directory inode.
const DIRECTORY: u8 = 1 << 2;
/// The file descriptor or file refers to a regular file inode.
const REGULAR_FILE: u8 = 1 << 3;
/// The file descriptor or file refers to a datagram socket.
const SOCKET_DGRAM: u8 = 1 << 4;
/// The file descriptor or file refers to a byte-stream socket.
const SOCKET_STREAM: u8 = 1 << 5;
/// The file refers to a symbolic link inode.
const SYMBOLIC_LINK: u8 = 1 << 6;

#[derive(Debug, Copy, Clone, Default, IntoBytes, KnownLayout, Immutable)]
#[repr(C)]
pub(crate) struct FileStat {
	pub dev: u64,
	pub ino: u64,
	pub filetype: u8,
	pub _pad0: u8,
	pub _pad1: u16,
	pub _pad2: u32,
	pub nlink: u64,
	pub size: u64,
	pub atim: u64,
	pub mtim: u64,
	pub ctim: u64,
}

#[derive(Debug, Copy, Clone, Default, IntoBytes, KnownLayout, Immutable)]
#[repr(C)]
pub(crate) struct FdStat {
	pub filetype: u8,
	pub _pad0: u8,
	pub fs_flags: u16,
	pub _pad1: u32,
	pub fs_rights_base: u64,
	pub fs_rights_inheriting: u64,
}

fn cvt(err: i32) -> i32 {
	match err {
		libc::EINVAL => ERRNO_INVAL.raw() as i32,
		libc::EFAULT => ERRNO_FAULT.raw() as i32,
		libc::ENOMEM => ERRNO_NOMEM.raw() as i32,
		_ => ERRNO_NOSYS.raw() as i32,
	}
}

pub(crate) fn init<T: 'static>(
	linker: &mut wasmtime::Linker<T>,
	module_and_args: &'static [OsString],
) -> Result<()> {
	debug!("Initialize module wasi_snapshot_preview1");

	{
		let mut guard = FD.lock().unwrap();
		guard.push(Descriptor::Stdin);
		guard.push(Descriptor::Stdout);
		guard.push(Descriptor::Stderr);
		guard.push(Descriptor::Directory(String::from("tmp")));
	}

	// In case WASI, it is required to emulate
	// https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md

	linker
		.func_wrap(
			"wasi_snapshot_preview1",
			"clock_time_get",
			|mut caller: Caller<'_, _>, clock_id: i32, _precision: i64, timestamp_ptr: i32| {
				match clock_id {
					0 => match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
						Ok(n) => {
							if let Some(Extern::Memory(mem)) = caller.get_export("memory") {
								let nanos = n.as_secs() * 1000000000 + n.subsec_nanos() as u64;
								let _ = mem.write(
									caller.as_context_mut(),
									timestamp_ptr.try_into().unwrap(),
									nanos.as_bytes(),
								);

								return ERRNO_SUCCESS.raw() as i32;
							}

							ERRNO_INVAL.raw() as i32
						}
						Err(_) => cvt(io::Error::last_os_error().raw_os_error().unwrap()),
					},
					1 => {
						static NOW: OnceLock<Instant> = OnceLock::new();

						if let Some(Extern::Memory(mem)) = caller.get_export("memory") {
							let elapsed = NOW.get_or_init(Instant::now).elapsed();
							let nanos: u64 =
								elapsed.as_secs() * 1000000000 + elapsed.subsec_nanos() as u64;
							let _ = mem.write(
								caller.as_context_mut(),
								timestamp_ptr.try_into().unwrap(),
								nanos.as_bytes(),
							);

							return ERRNO_SUCCESS.raw() as i32;
						}

						ERRNO_INVAL.raw() as i32
					}
					_ => ERRNO_INVAL.raw() as i32,
				}
			},
		)
		.unwrap();
	linker
		.func_wrap(
			"wasi_snapshot_preview1",
			"args_get",
			|mut caller: Caller<'_, _>, argv_ptr: i32, argv_buf_ptr: i32| {
				if let Some(Extern::Memory(mem)) = caller.get_export("memory") {
					let mut pos: u32 = argv_buf_ptr as u32;
					for (i, element) in module_and_args.iter().enumerate() {
						let _ = mem.write(
							caller.as_context_mut(),
							(argv_ptr + (i * size_of::<u32>()) as i32)
								.try_into()
								.unwrap(),
							pos.as_bytes(),
						);

						let mut arg = element.clone().into_encoded_bytes();
						arg.push(0); // plus null terminator

						let _ = mem.write(
							caller.as_context_mut(),
							pos.try_into().unwrap(),
							arg.as_bytes(),
						);

						pos += arg.len() as u32;
					}
				}
				ERRNO_SUCCESS.raw() as i32
			},
		)
		.unwrap();
	linker
		.func_wrap(
			"wasi_snapshot_preview1",
			"poll_oneoff",
			|mut caller: Caller<'_, _>,
			 input: i32,
			 output: i32,
			 nsubscriptions: i32,
			 nevents: i32| {
				if nsubscriptions == 0 {
					return ERRNO_INVAL.raw() as i32;
				}

				if let Some(Extern::Memory(mem)) = caller.get_export("memory") {
					for _i in 0..nsubscriptions {
						let mut event: MaybeUninit<Subscription> =
							unsafe { MaybeUninit::zeroed().assume_init() };

						let _ =
							mem.read(caller.as_context_mut(), input.try_into().unwrap(), unsafe {
								std::mem::transmute::<
									&mut Subscription,
									&mut [u8; size_of::<Subscription>()],
								>(event.assume_init_mut())
							});

						// currently, only the event SubscriptionClock is supported
						assert!(unsafe { event.assume_init().u.u.clock.id } == CLOCKID_MONOTONIC);
						let duration =
							Duration::from_nanos(unsafe { event.assume_init().u.u.clock.timeout });
						thread::sleep(duration);

						const USERDATA: wasi::Userdata = 0x0123_45678;
						let result = Event {
							userdata: USERDATA,
							error: ERRNO_SUCCESS,
							type_: EVENTTYPE_CLOCK,
							fd_readwrite: EventFdReadwrite {
								nbytes: 0,
								flags: 0,
							},
						};

						let _ = mem.write(
							caller.as_context_mut(),
							output.try_into().unwrap(),
							unsafe {
								std::slice::from_raw_parts(
									(&result as *const _) as *const u8,
									size_of::<Event>(),
								)
							},
						);
					}

					let result: u32 = nsubscriptions.try_into().unwrap();
					let _ = mem.write(
						caller.as_context_mut(),
						nevents.try_into().unwrap(),
						unsafe {
							std::slice::from_raw_parts(
								(&result as *const _) as *const u8,
								size_of::<u32>(),
							)
						},
					);
				}

				ERRNO_SUCCESS.raw() as i32
			},
		)
		.unwrap();
	linker
		.func_wrap("wasi_snapshot_preview1", "sched_yield", || {
			std::thread::yield_now();
			ERRNO_SUCCESS.raw() as i32
		})
		.unwrap();
	linker
		.func_wrap(
			"wasi_snapshot_preview1",
			"path_open",
			|mut caller: Caller<'_, _>,
			 _fd: i32,
			 _dirflags: i32,
			 path_ptr: i32,
			 path_len: i32,
			 oflags: i32,
			 _fs_rights_base: Rights,
			 _fs_rights_inheriting: Rights,
			 _fdflags: i32,
			 fd_ptr: i32| {
				let oflags = Oflags::from_bits(oflags).unwrap();
				if let Some(Extern::Memory(mem)) = caller.get_export("memory") {
					let mut path = vec![0u8; path_len.try_into().unwrap()];

					let _ = mem.read(
						caller.as_context_mut(),
						path_ptr.try_into().unwrap(),
						path.as_mut_bytes(),
					);
					let path = "/".to_owned() + std::str::from_utf8(&path).unwrap();

					let mut flags: i32 = 0;
					if oflags.contains(Oflags::OFLAGS_CREAT) {
						flags |= libc::O_CREAT;
					}
					if oflags.contains(Oflags::OFLAGS_TRUNC) {
						flags |= libc::O_TRUNC;
					}
					flags |= libc::O_RDWR;

					let mut c_path = vec![0u8; path.len() + 1];
					c_path[..path.len()].copy_from_slice(path.as_bytes());
					{
						let raw_fd =
							unsafe { libc::open(c_path.as_ptr() as *const c_char, flags, 0) };
						let mut guard = FD.lock().unwrap();
						for (i, entry) in guard.iter_mut().enumerate() {
							if entry.is_none() {
								*entry = Descriptor::File(FileStream { raw_fd, path });
								let _ = mem.write(
									caller.as_context_mut(),
									fd_ptr.try_into().unwrap(),
									i.as_bytes(),
								);

								return ERRNO_SUCCESS.raw() as i32;
							}
						}
						guard.push(Descriptor::File(FileStream { raw_fd, path }));

						let new_fd: i32 = (guard.len() - 1).try_into().unwrap();
						let _ = mem.write(
							caller.as_context_mut(),
							fd_ptr.try_into().unwrap(),
							new_fd.as_bytes(),
						);
					}

					return ERRNO_SUCCESS.raw() as i32;
				}

				ERRNO_INVAL.raw() as i32
			},
		)
		.unwrap();
	linker
		.func_wrap(
			"wasi_snapshot_preview1",
			"path_unlink_file",
			|mut caller: Caller<'_, _>, _fd: i32, path_ptr: i32, path_len: i32| {
				if let Some(Extern::Memory(mem)) = caller.get_export("memory") {
					let mut path = vec![0u8; path_len.try_into().unwrap()];

					let _ = mem.read(
						caller.as_context_mut(),
						path_ptr.try_into().unwrap(),
						path.as_mut_bytes(),
					);

					let path = "/".to_owned() + std::str::from_utf8(&path).unwrap();
					std::fs::remove_file(path).unwrap();
				}

				ERRNO_SUCCESS.raw() as i32
			},
		)
		.unwrap();
	linker
		.func_wrap(
			"wasi_snapshot_preview1",
			"fd_prestat_get",
			|mut caller: Caller<'_, _>, fd: i32, prestat_ptr: i32| {
				let guard = FD.lock().unwrap();
				if fd < guard.len().try_into().unwrap()
					&& let Some(Extern::Memory(mem)) = caller.get_export("memory")
					&& let Descriptor::Directory(name) = &guard[fd as usize]
				{
					let stat = Prestat {
						tag: PREOPENTYPE_DIR.raw(),
						u: PrestatU {
							dir: PrestatDir {
								pr_name_len: name.len(),
							},
						},
					};

					let _ = mem.write(
						caller.as_context_mut(),
						prestat_ptr.try_into().unwrap(),
						unsafe {
							std::slice::from_raw_parts(
								(&stat as *const _) as *const u8,
								size_of::<Prestat>(),
							)
						},
					);

					return ERRNO_SUCCESS.raw() as i32;
				}

				ERRNO_BADF.raw() as i32
			},
		)
		.unwrap();
	linker
		.func_wrap(
			"wasi_snapshot_preview1",
			"fd_tell",
			|mut caller: Caller<'_, _>, fd: i32, offset_ptr: i32| {
				let guard = FD.lock().unwrap();
				if fd < guard.len().try_into().unwrap()
					&& let Descriptor::File(file) = &guard[fd as usize]
					&& let Some(Extern::Memory(mem)) = caller.get_export("memory")
				{
					let offset = unsafe { libc::lseek(file.raw_fd, 0, libc::SEEK_CUR) };

					if offset > 0 {
						let _ = mem.write(
							caller.as_context_mut(),
							offset_ptr.try_into().unwrap(),
							offset.as_bytes(),
						);

						return ERRNO_SUCCESS.raw() as i32;
					}
				}

				ERRNO_BADF.raw() as i32
			},
		)
		.unwrap();
	linker
		.func_wrap(
			"wasi_snapshot_preview1",
			"fd_prestat_dir_name",
			|mut caller: Caller<'_, _>, fd: i32, path_ptr: i32, path_len: i32| {
				let guard = FD.lock().unwrap();
				if fd < guard.len().try_into().unwrap()
					&& let Descriptor::Directory(path) = &guard[fd as usize]
				{
					if let Some(Extern::Memory(mem)) = caller.get_export(
						"memory
",
					) {
						if path_len < path.len().try_into().unwrap() {
							return ERRNO_INVAL.raw() as i32;
						}

						let _ = mem.write(
							caller.as_context_mut(),
							path_ptr.try_into().unwrap(),
							path.as_bytes(),
						);
					}

					return ERRNO_SUCCESS.raw() as i32;
				}

				ERRNO_BADF.raw() as i32
			},
		)
		.unwrap();
	linker
		.func_wrap("wasi_snapshot_preview1", "fd_close", |fd: i32| {
			let mut guard = FD.lock().unwrap();
			if fd < guard.len().try_into().unwrap()
				&& let Descriptor::File(file) = &guard[fd as usize]
			{
				unsafe {
					libc::close(file.raw_fd);
				}
				guard[fd as usize] = Descriptor::None;
			}

			ERRNO_SUCCESS.raw() as i32
		})
		.unwrap();
	linker
		.func_wrap(
			"wasi_snapshot_preview1",
			"fd_fdstat_get",
			|mut caller: Caller<'_, _>, fd: i32, fdstat_ptr: i32| {
				let guard = FD.lock().unwrap();
				if fd < guard.len().try_into().unwrap() {
					let fdstat = match &guard[fd as usize] {
						Descriptor::Stdin | Descriptor::Stdout | Descriptor::Stderr => FdStat {
							filetype: CHARACTER_DEVICE,
							..Default::default()
						},
						Descriptor::File(_) => FdStat {
							filetype: REGULAR_FILE,
							..Default::default()
						},
						Descriptor::Directory(_) => FdStat {
							filetype: DIRECTORY,
							..Default::default()
						},
						_ => {
							return ERRNO_INVAL.raw() as i32;
						}
					};

					if let Some(Extern::Memory(mem)) = caller.get_export("memory") {
						let _ = mem.write(
							caller.as_context_mut(),
							fdstat_ptr.try_into().unwrap(),
							fdstat.as_bytes(),
						);

						return ERRNO_SUCCESS.raw() as i32;
					}

					ERRNO_INVAL.raw() as i32
				} else {
					ERRNO_INVAL.raw() as i32
				}
			},
		)
		.unwrap();
	linker
		.func_wrap(
			"wasi_snapshot_preview1",
			"fd_fdstat_set_flags",
			|_fd: i32, _fdflags: i32| i32::from(ERRNO_NOSYS.raw()),
		)
		.unwrap();
	linker
		.func_wrap(
			"wasi_snapshot_preview1",
			"fd_filestat_get",
			|mut caller: Caller<'_, _>, fd: i32, filestat_ptr: i32| {
				let guard = FD.lock().unwrap();
				if fd >= guard.len().try_into().unwrap() {
					return ERRNO_INVAL.raw() as i32;
				}

				if let Descriptor::File(file) = &guard[fd as usize] {
					let metadata = std::fs::metadata(file.path.clone()).unwrap();
					let filestat = FileStat {
						filetype: REGULAR_FILE,
						size: metadata.len(),
						mtim: metadata
							.modified()
							.unwrap()
							.duration_since(UNIX_EPOCH)
							.unwrap()
							.as_nanos()
							.try_into()
							.unwrap(),
						atim: metadata
							.accessed()
							.unwrap()
							.duration_since(UNIX_EPOCH)
							.unwrap()
							.as_nanos()
							.try_into()
							.unwrap(),
						ctim: metadata
							.created()
							.unwrap()
							.duration_since(UNIX_EPOCH)
							.unwrap()
							.as_nanos()
							.try_into()
							.unwrap(),
						..Default::default()
					};

					if let Some(Extern::Memory(mem)) = caller.get_export("memory") {
						let _ = mem.write(
							caller.as_context_mut(),
							filestat_ptr.try_into().unwrap(),
							filestat.as_bytes(),
						);

						return ERRNO_SUCCESS.raw() as i32;
					}
				}

				ERRNO_INVAL.raw() as i32
			},
		)
		.unwrap();
	linker
		.func_wrap(
			"wasi_snapshot_preview1",
			"fd_read",
			|mut caller: Caller<'_, _>, fd: i32, iovs_ptr: i32, iovs_len: i32, nread_ptr: i32| {
				let fd = if fd <= 2 {
					fd
				} else {
					let guard = FD.lock().unwrap();
					if fd >= guard.len().try_into().unwrap() {
						return ERRNO_INVAL.raw() as i32;
					}

					if let Descriptor::File(file) = &guard[fd as usize] {
						file.raw_fd
					} else {
						return ERRNO_INVAL.raw() as i32;
					}
				};

				if let Some(Extern::Memory(mem)) = caller.get_export("memory") {
					let mut iovs = vec![0i32; (2 * iovs_len).try_into().unwrap()];
					let _ = mem.read(
						caller.as_context(),
						iovs_ptr.try_into().unwrap(),
						iovs.as_mut_bytes(),
					);

					let mut nread_bytes: i32 = 0;
					let mut i = 0;
					while i < iovs.len() {
						let len = iovs[i + 1];
						let mut data: Vec<MaybeUninit<u8>> =
							Vec::with_capacity(len.try_into().unwrap());
						unsafe {
							data.set_len(len as usize);
						}

						let result = unsafe {
							libc::read(
								fd,
								data.assume_init_mut().as_mut_ptr().cast(),
								len.try_into().unwrap(),
							)
						};

						match result.cmp(&0) {
							Ordering::Greater => {
								let _ = mem.write(
									caller.as_context_mut(),
									iovs[i].try_into().unwrap(),
									unsafe { data[..result as usize].assume_init_ref() },
								);

								nread_bytes += result as i32;
								if result < len.try_into().unwrap() {
									break;
								}
							}
							Ordering::Equal => {
								if result < len.try_into().unwrap() {
									break;
								}
							}
							Ordering::Less => {
								return (-result).try_into().unwrap();
							}
						}

						i += 2;
					}

					let _ = mem.write(
						caller.as_context_mut(),
						nread_ptr.try_into().unwrap(),
						nread_bytes.as_bytes(),
					);

					return ERRNO_SUCCESS.raw() as i32;
				}

				ERRNO_INVAL.raw() as i32
			},
		)
		.unwrap();
	linker
		.func_wrap(
			"wasi_snapshot_preview1",
			"fd_seek",
			|mut caller: Caller<'_, _>, fd: i32, offset: i64, whence: i32, pos_ptr: i32| {
				let fd = if fd <= 2 {
					fd
				} else {
					let guard = FD.lock().unwrap();
					if fd >= guard.len().try_into().unwrap() {
						return ERRNO_INVAL.raw() as i32;
					}

					if let Descriptor::File(file) = &guard[fd as usize] {
						file.raw_fd
					} else {
						return ERRNO_INVAL.raw() as i32;
					}
				};

				#[allow(clippy::useless_conversion)]
				let result = unsafe { libc::lseek(fd, offset.try_into().unwrap(), whence) };

				if let Some(Extern::Memory(mem)) = caller.get_export("memory") {
					let _ = mem.write(
						caller.as_context_mut(),
						pos_ptr.try_into().unwrap(),
						result.as_bytes(),
					);
				}

				ERRNO_SUCCESS.raw() as i32
			},
		)
		.unwrap();
	linker
		.func_wrap(
			"wasi_snapshot_preview1",
			"fd_write",
			|mut caller: Caller<'_, _>,
			 fd: i32,
			 iovs_ptr: i32,
			 iovs_len: i32,
			 nwritten_ptr: i32| {
				let fd = if fd <= 2 {
					fd
				} else {
					let guard = FD.lock().unwrap();
					if fd >= guard.len().try_into().unwrap() {
						return ERRNO_INVAL.raw() as i32;
					}

					if let Descriptor::File(file) = &guard[fd as usize] {
						file.raw_fd
					} else {
						return ERRNO_INVAL.raw() as i32;
					}
				};

				if let Some(Extern::Memory(mem)) = caller.get_export("memory") {
					let mut iovs = vec![0i32; (2 * iovs_len).try_into().unwrap()];
					let _ = mem.read(
						caller.as_context(),
						iovs_ptr.try_into().unwrap(),
						iovs.as_mut_bytes(),
					);

					let mut nwritten_bytes: i32 = 0;
					let mut i = 0;
					while i < iovs.len() {
						let len = iovs[i + 1];

						// len = 0 => ignore entry nothing to write
						if len == 0 {
							i += 2;
							continue;
						}

						let mut data: Vec<MaybeUninit<u8>> =
							Vec::with_capacity(len.try_into().unwrap());
						unsafe {
							data.set_len(len as usize);
						}

						let _ =
							mem.read(caller.as_context(), iovs[i].try_into().unwrap(), unsafe {
								data.assume_init_mut()
							});
						let result = unsafe {
							libc::write(
								fd,
								data.assume_init_ref().as_ptr().cast(),
								len.try_into().unwrap(),
							)
						};

						if result >= 0 {
							nwritten_bytes += result as i32;
							if result < len.try_into().unwrap() {
								break;
							}
						} else {
							return (-result).try_into().unwrap();
						}

						i += 2;
					}

					let _ = mem.write(
						caller.as_context_mut(),
						nwritten_ptr.try_into().unwrap(),
						nwritten_bytes.as_bytes(),
					);

					return ERRNO_SUCCESS.raw() as i32;
				}

				ERRNO_INVAL.raw() as i32
			},
		)
		.unwrap();
	linker
		.func_wrap(
			"wasi_snapshot_preview1",
			"args_sizes_get",
			move |mut caller: Caller<'_, _>, number_args_ptr: i32, args_size_ptr: i32| {
				let nargs: u32 = module_and_args.len().try_into().unwrap();
				// Currently, we ignore the arguments
				if let Some(Extern::Memory(mem)) = caller.get_export("memory") {
					let _ = mem.write(
						caller.as_context_mut(),
						number_args_ptr.try_into().unwrap(),
						nargs.as_bytes(),
					);

					let nargs_size: u32 = module_and_args
						.iter()
						.fold(0, |acc, arg| {
							acc + arg.len() + 1 // +1 for the null terminator
						})
						.try_into()
						.unwrap();
					let _ = mem.write(
						caller.as_context_mut(),
						args_size_ptr.try_into().unwrap(),
						nargs_size.as_bytes(),
					);

					return ERRNO_SUCCESS.raw() as i32;
				}

				ERRNO_INVAL.raw() as i32
			},
		)
		.unwrap();
	linker
		.func_wrap(
			"wasi_snapshot_preview1",
			"environ_get",
			|mut caller: Caller<'_, _>, env_ptr: i32, env_buffer_ptr: i32| {
				if let Some(Extern::Memory(mem)) = caller.get_export("memory") {
					let mut pos: u32 = env_buffer_ptr as u32;
					for (i, (key, value)) in std::env::vars().enumerate() {
						let _ = mem.write(
							caller.as_context_mut(),
							(env_ptr + (i * size_of::<u32>()) as i32)
								.try_into()
								.unwrap(),
							pos.as_bytes(),
						);

						let mut env = key;
						env.push('=');
						env.push_str(&value);
						let env = unsafe { env.as_mut_vec() };
						env.push(0); // plus null terminator

						let _ = mem.write(
							caller.as_context_mut(),
							pos.try_into().unwrap(),
							env.as_bytes(),
						);

						pos += env.len() as u32;
					}
				}
				ERRNO_SUCCESS.raw() as i32
			},
		)
		.unwrap();
	linker
		.func_wrap(
			"wasi_snapshot_preview1",
			"environ_sizes_get",
			|mut caller: Caller<'_, _>, number_env_variables_ptr: i32, env_buffer_size_ptr: i32| {
				if let Some(Extern::Memory(mem)) = caller.get_export("memory") {
					let mut env_buffer_size: u32 = 0;
					let mut nnumber_env_variables: u32 = 0;

					for (key, value) in std::env::vars() {
						nnumber_env_variables += 1;
						env_buffer_size += u32::try_from(key.len() + value.len() + 2).unwrap(); // +2 for the null terminator and '='
					}

					let _ = mem.write(
						caller.as_context_mut(),
						number_env_variables_ptr.try_into().unwrap(),
						nnumber_env_variables.as_bytes(),
					);
					let _ = mem.write(
						caller.as_context_mut(),
						env_buffer_size_ptr.try_into().unwrap(),
						env_buffer_size.as_bytes(),
					);

					return ERRNO_SUCCESS.raw() as i32;
				}

				ERRNO_INVAL.raw() as i32
			},
		)
		.unwrap();
	linker
		.func_wrap("wasi_snapshot_preview1", "proc_exit", |_: i32| {
			error!("Panic in WASM module")
		})
		.unwrap();

	Ok(())
}
