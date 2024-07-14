use std::sync::{Mutex, OnceLock};
use std::time::{Instant, SystemTime};

use anyhow::Result;
use bitflags::bitflags;
use log::debug;
use wasi::*;
use wasmtime::{AsContext, AsContextMut, Caller, Extern};
use zerocopy::AsBytes;

static FD: Mutex<Vec<Descriptor>> = Mutex::new(Vec::new());

#[derive(Debug, Clone, PartialEq)]
struct FileStream {
	pub raw_fd: i32,
}

#[derive(Debug, Clone, PartialEq)]
enum Descriptor {
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

fn cvt(err: i32) -> i32 {
	match err {
		hermit_abi::EINVAL => ERRNO_INVAL.raw() as i32,
		hermit_abi::EFAULT => ERRNO_FAULT.raw() as i32,
		hermit_abi::ENOMEM => ERRNO_NOMEM.raw() as i32,
		_ => ERRNO_NOSYS.raw() as i32,
	}
}

pub(crate) fn init<T>(linker: &mut wasmtime::Linker<T>) -> Result<()> {
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
						Err(_) => unsafe { cvt(hermit_abi::get_errno()) },
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
						path.as_bytes_mut(),
					);
					let path = "/".to_owned() + std::str::from_utf8(&path).unwrap();

					let mut flags: i32 = 0;
					if oflags.contains(Oflags::OFLAGS_CREAT) {
						flags |= hermit_abi::O_CREAT;
					}
					/*if oflags.contains(Oflags::OFLAGS_TRUNC) {
						flags |= hermit_abi::O_TRUNC;
					}*/
					flags |= hermit_abi::O_RDWR;

					let mut c_path = vec![0u8; path.len() + 1];
					c_path[..path.len()].copy_from_slice(path.as_bytes());
					{
						let raw_fd =
							unsafe { hermit_abi::open(c_path.as_ptr() as *const i8, flags, 0) };
						let mut guard = FD.lock().unwrap();
						for (i, entry) in guard.iter_mut().enumerate() {
							if entry.is_none() {
								*entry = Descriptor::File(FileStream { raw_fd });
								let _ = mem.write(
									caller.as_context_mut(),
									fd_ptr.try_into().unwrap(),
									i.as_bytes(),
								);

								return ERRNO_SUCCESS.raw() as i32;
							}
						}
						guard.push(Descriptor::File(FileStream { raw_fd }));

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
						path.as_bytes_mut(),
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
				if fd < guard.len().try_into().unwrap() {
					if let Some(Extern::Memory(mem)) = caller.get_export("memory") {
						if let Descriptor::Directory(name) = &guard[fd as usize] {
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
				if fd < guard.len().try_into().unwrap() {
					if let Descriptor::Directory(path) = &guard[fd as usize] {
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
				}

				ERRNO_BADF.raw() as i32
			},
		)
		.unwrap();
	linker
		.func_wrap("wasi_snapshot_preview1", "fd_close", |fd: i32| {
			let mut guard = FD.lock().unwrap();
			if fd < guard.len().try_into().unwrap() {
				if let Descriptor::File(file) = &guard[fd as usize] {
					unsafe {
						hermit_abi::close(file.raw_fd);
					}
					guard[fd as usize] = Descriptor::None;
				}
			}

			ERRNO_SUCCESS.raw() as i32
		})
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
						iovs.as_bytes_mut(),
					);

					let mut nwritten_bytes: i32 = 0;
					let mut i = 0;
					while i < iovs.len() {
						let len = iovs[i + 1];
						let mut data = vec![0u8; len.try_into().unwrap()];

						let _ = mem.read(
							caller.as_context(),
							iovs[i].try_into().unwrap(),
							&mut data[..],
						);
						let result = unsafe {
							hermit_abi::write(fd, data.as_ptr(), len.try_into().unwrap())
						};
						if result > 0 {
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
			|mut caller: Caller<'_, _>, number_args_ptr: i32, args_size_ptr: i32| {
				// Currently, we ignore the arguments
				if let Some(Extern::Memory(mem)) = caller.get_export("memory") {
					// Currently, we ignore the environment
					let zero: u32 = 0;

					let _ = mem.write(
						caller.as_context_mut(),
						number_args_ptr.try_into().unwrap(),
						zero.as_bytes(),
					);
					let _ = mem.write(
						caller.as_context_mut(),
						args_size_ptr.try_into().unwrap(),
						zero.as_bytes(),
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
			|_env_ptr: i32, _env_buffer_ptr: i32| ERRNO_INVAL.raw() as i32,
		)
		.unwrap();
	linker
		.func_wrap(
			"wasi_snapshot_preview1",
			"environ_sizes_get",
			|mut caller: Caller<'_, _>, number_env_variables_ptr: i32, env_buffer_size_ptr: i32| {
				if let Some(Extern::Memory(mem)) = caller.get_export("memory") {
					// Currently, we ignore the environment
					let zero: u32 = 0;

					let _ = mem.write(
						caller.as_context_mut(),
						number_env_variables_ptr.try_into().unwrap(),
						zero.as_bytes(),
					);
					let _ = mem.write(
						caller.as_context_mut(),
						env_buffer_size_ptr.try_into().unwrap(),
						zero.as_bytes(),
					);

					return ERRNO_SUCCESS.raw() as i32;
				}

				ERRNO_INVAL.raw() as i32
			},
		)
		.unwrap();
	linker
		.func_wrap("wasi_snapshot_preview1", "proc_exit", |_: i32| {
			panic!("Panic in WASM module")
		})
		.unwrap();

	Ok(())
}
