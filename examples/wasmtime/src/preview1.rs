use std::sync::OnceLock;
use std::time::{Instant, SystemTime};

use anyhow::Result;
use log::info;
use wasi::*;
use wasmtime::{AsContext, AsContextMut, Caller, Extern};
use zerocopy::AsBytes;

fn cvt(err: i32) -> i32 {
	match err {
		hermit_abi::EINVAL => ERRNO_INVAL.raw() as i32,
		hermit_abi::EFAULT => ERRNO_FAULT.raw() as i32,
		hermit_abi::ENOMEM => ERRNO_NOMEM.raw() as i32,
		_ => ERRNO_NOSYS.raw() as i32,
	}
}

pub(crate) fn init<T>(linker: &mut wasmtime::Linker<T>) -> Result<()> {
	info!("Initialize module wasi_snapshot_preview1");

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
			"fd_write",
			|mut caller: Caller<'_, _>,
			 fd: i32,
			 iovs_ptr: i32,
			 iovs_len: i32,
			 nwritten_ptr: i32| {
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
