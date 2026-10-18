use crate::preview2::WasiCtx;
use crate::preview2::bindings::random::{insecure, insecure_seed, random};

fn fill(buf: &mut [u8]) -> wasmtime::Result<()> {
	cfg_if::cfg_if! {
		if #[cfg(target_os = "hermit")] {
			let mut filled = 0;
			while filled < buf.len() {
				let ret = unsafe {
					hermit_abi::read_entropy(buf[filled..].as_mut_ptr(), buf.len() - filled, 0)
				};
				if ret <= 0 {
					wasmtime::bail!("unable to read entropy: {ret}");
				}
				filled += ret as usize;
			}
		} else {
			for chunk in buf.chunks_mut(256) {
				if unsafe { libc::getentropy(chunk.as_mut_ptr().cast(), chunk.len()) } != 0 {
					wasmtime::bail!("unable to read entropy: {}", std::io::Error::last_os_error());
				}
			}
		}
	}
	Ok(())
}

fn random_bytes(len: u64) -> wasmtime::Result<Vec<u8>> {
	let mut buf = vec![0; len.try_into()?];
	fill(&mut buf)?;
	Ok(buf)
}

fn random_u64() -> wasmtime::Result<u64> {
	let mut buf = [0; 8];
	fill(&mut buf)?;
	Ok(u64::from_ne_bytes(buf))
}

impl random::Host for WasiCtx {
	fn get_random_bytes(&mut self, len: u64) -> wasmtime::Result<Vec<u8>> {
		random_bytes(len)
	}

	fn get_random_u64(&mut self) -> wasmtime::Result<u64> {
		random_u64()
	}
}

impl insecure::Host for WasiCtx {
	fn get_insecure_random_bytes(&mut self, len: u64) -> wasmtime::Result<Vec<u8>> {
		random_bytes(len)
	}

	fn get_insecure_random_u64(&mut self) -> wasmtime::Result<u64> {
		random_u64()
	}
}

impl insecure_seed::Host for WasiCtx {
	fn insecure_seed(&mut self) -> wasmtime::Result<(u64, u64)> {
		Ok((random_u64()?, random_u64()?))
	}
}
