use std::fmt::Write as _;

use axum::extract::Query;
use axum::routing::get;
use axum::Router;
#[cfg(target_os = "hermit")]
use hermit as _;
use serde::Deserialize;
use tokio::{io, net};

#[tokio::main(flavor = "current_thread")]
async fn main() -> io::Result<()> {
	println!("Testing axum...");

	let app = Router::new()
		.route("/", get(root))
		.route("/blk", get(blk))
		.route("/bytes", get(bytes));

	#[cfg(not(target_family = "wasm"))]
	let listener = net::TcpListener::bind("0.0.0.0:9975").await?;
	// tokio does not support binding sockets on WASI.
	#[cfg(target_family = "wasm")]
	let listener = {
		let listener = std::net::TcpListener::bind("0.0.0.0:9975")?;
		listener.set_nonblocking(true)?;
		net::TcpListener::from_std(listener)?
	};
	axum::serve(listener, app).await?;

	Ok(())
}

async fn root() -> &'static str {
	"Hello, world!\n"
}

async fn blk() -> String {
	// Spawning a blocking task failed on WASI, so we only run this test on non-WASI targets.
	tokio::task::spawn_blocking(|| "blocking ok\n".to_string())
		.await
		.unwrap()
}

#[derive(Deserialize)]
struct Bytes {
	len: usize,
}

async fn bytes(Query(Bytes { len }): Query<Bytes>) -> String {
	const SEPARATOR: &str = ", ";
	const USIZE_BITS: usize = usize::BITS as usize;
	const BITS_PER_HEX_DIGIT: usize = 4;

	let mut s = String::with_capacity(len);
	let mut buffer = String::with_capacity(SEPARATOR.len() + (USIZE_BITS / BITS_PER_HEX_DIGIT));
	let mut range = 0usize..;
	let mut remaining_len;

	loop {
		let i = range.next().unwrap();
		if i != 0 {
			buffer.push_str(SEPARATOR);
		}
		write!(&mut buffer, "{i:x}").unwrap();
		remaining_len = s.capacity() - s.len();

		if remaining_len <= buffer.len() {
			break;
		}

		s.extend(buffer.drain(..));
	}

	s.push_str(&"x".repeat(remaining_len - 1));
	s.push('X');

	s
}
