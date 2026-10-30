mod generated {
	wasmtime::component::bindgen!({
		path: "wit",
		world: "hermit:wasm/command",
		imports: { default: trappable },
		trappable_error_type: {
			"wasi:io/streams.stream-error" => wasmtime_wasi_io::streams::StreamError,
			"wasi:filesystem/types.error-code" => crate::preview2::filesystem::FsError,
			"wasi:sockets/network.error-code" => crate::preview2::sockets::SocketError,
		},
		with: {
			"wasi:io/error": wasmtime_wasi_io::bindings::wasi::io::error,
			"wasi:io/poll.pollable": wasmtime_wasi_io::poll::DynPollable,
			"wasi:io/streams.input-stream": wasmtime_wasi_io::streams::DynInputStream,
			"wasi:io/streams.output-stream": wasmtime_wasi_io::streams::DynOutputStream,
			"wasi:cli/terminal-input.terminal-input": crate::preview2::cli::TerminalInput,
			"wasi:cli/terminal-output.terminal-output": crate::preview2::cli::TerminalOutput,
			"wasi:filesystem/types.descriptor": crate::preview2::filesystem::Descriptor,
			"wasi:filesystem/types.directory-entry-stream": crate::preview2::filesystem::DirectoryEntryStream,
			"wasi:sockets/network.network": crate::preview2::sockets::Network,
			"wasi:sockets/tcp.tcp-socket": crate::preview2::sockets::TcpSocket,
		},
	});
}

pub use self::generated::Command;
pub use self::generated::wasi::*;
