use std::io::{Read, Write};

use wasmtime::component::Resource;
use wasmtime_wasi_io::bytes::Bytes;
use wasmtime_wasi_io::poll::Pollable;
use wasmtime_wasi_io::streams::{
	DynInputStream, DynOutputStream, InputStream, OutputStream, StreamError, StreamResult,
};

use crate::preview2::bindings::cli::{
	environment, exit, stderr, stdin, stdout, terminal_input, terminal_output, terminal_stderr,
	terminal_stdin, terminal_stdout,
};
use crate::preview2::{I32Exit, WasiCtx};

pub struct TerminalInput;
pub struct TerminalOutput;

struct Stdin;

#[wasmtime_wasi_io::async_trait]
impl Pollable for Stdin {
	async fn ready(&mut self) {}
}

impl InputStream for Stdin {
	fn read(&mut self, size: usize) -> StreamResult<Bytes> {
		let mut buf = vec![0; size];
		match std::io::stdin().read(&mut buf) {
			Ok(0) if size > 0 => Err(StreamError::Closed),
			Ok(n) => {
				buf.truncate(n);
				Ok(buf.into())
			}
			Err(e) => Err(StreamError::LastOperationFailed(e.into())),
		}
	}
}

enum Stdio {
	Stdout,
	Stderr,
}

#[wasmtime_wasi_io::async_trait]
impl Pollable for Stdio {
	async fn ready(&mut self) {}
}

impl OutputStream for Stdio {
	fn write(&mut self, bytes: Bytes) -> StreamResult<()> {
		let result = match self {
			Stdio::Stdout => std::io::stdout().write_all(&bytes),
			Stdio::Stderr => std::io::stderr().write_all(&bytes),
		};
		result.map_err(|e| StreamError::LastOperationFailed(e.into()))
	}

	fn flush(&mut self) -> StreamResult<()> {
		let result = match self {
			Stdio::Stdout => std::io::stdout().flush(),
			Stdio::Stderr => std::io::stderr().flush(),
		};
		result.map_err(|e| StreamError::LastOperationFailed(e.into()))
	}

	fn check_write(&mut self) -> StreamResult<usize> {
		Ok(64 * 1024)
	}
}

impl environment::Host for WasiCtx {
	fn get_environment(&mut self) -> wasmtime::Result<Vec<(String, String)>> {
		Ok(self.env.clone())
	}

	fn get_arguments(&mut self) -> wasmtime::Result<Vec<String>> {
		Ok(self.args.clone())
	}

	fn initial_cwd(&mut self) -> wasmtime::Result<Option<String>> {
		Ok(None)
	}
}

impl exit::Host for WasiCtx {
	fn exit(&mut self, status: Result<(), ()>) -> wasmtime::Result<()> {
		let code = if status.is_ok() { 0 } else { 1 };
		Err(wasmtime::Error::new(I32Exit(code)))
	}

	fn exit_with_code(&mut self, status_code: u8) -> wasmtime::Result<()> {
		Err(wasmtime::Error::new(I32Exit(status_code.into())))
	}
}

impl stdin::Host for WasiCtx {
	fn get_stdin(&mut self) -> wasmtime::Result<Resource<DynInputStream>> {
		let stream: DynInputStream = Box::new(Stdin);
		Ok(self.table.push(stream)?)
	}
}

impl stdout::Host for WasiCtx {
	fn get_stdout(&mut self) -> wasmtime::Result<Resource<DynOutputStream>> {
		let stream: DynOutputStream = Box::new(Stdio::Stdout);
		Ok(self.table.push(stream)?)
	}
}

impl stderr::Host for WasiCtx {
	fn get_stderr(&mut self) -> wasmtime::Result<Resource<DynOutputStream>> {
		let stream: DynOutputStream = Box::new(Stdio::Stderr);
		Ok(self.table.push(stream)?)
	}
}

impl terminal_input::Host for WasiCtx {}

impl terminal_input::HostTerminalInput for WasiCtx {
	fn drop(&mut self, r: Resource<TerminalInput>) -> wasmtime::Result<()> {
		self.table.delete(r)?;
		Ok(())
	}
}

impl terminal_output::Host for WasiCtx {}

impl terminal_output::HostTerminalOutput for WasiCtx {
	fn drop(&mut self, r: Resource<TerminalOutput>) -> wasmtime::Result<()> {
		self.table.delete(r)?;
		Ok(())
	}
}

impl terminal_stdin::Host for WasiCtx {
	fn get_terminal_stdin(&mut self) -> wasmtime::Result<Option<Resource<TerminalInput>>> {
		Ok(None)
	}
}

impl terminal_stdout::Host for WasiCtx {
	fn get_terminal_stdout(&mut self) -> wasmtime::Result<Option<Resource<TerminalOutput>>> {
		Ok(None)
	}
}

impl terminal_stderr::Host for WasiCtx {
	fn get_terminal_stderr(&mut self) -> wasmtime::Result<Option<Resource<TerminalOutput>>> {
		Ok(None)
	}
}
