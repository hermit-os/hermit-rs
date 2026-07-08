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
		.route("/bytes", get(bytes));

	let listener = net::TcpListener::bind("0.0.0.0:9975").await?;
	axum::serve(listener, app).await?;

	Ok(())
}

async fn root() -> &'static str {
	"Hello, world!\n"
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
