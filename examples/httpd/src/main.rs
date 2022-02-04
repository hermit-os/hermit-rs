/// Example is derived from tiny-http example
/// https://github.com/tiny-http/tiny-http/blob/master/examples/hello-world.rs
use ascii::AsciiString;
#[cfg(target_os = "hermit")]
use hermit_sys as _;

fn main() {
	let crab = vec![0xF0_u8, 0x9F_u8, 0xA6_u8, 0x80_u8];
	let text = format!(
		"Hello from RustyHermit {}",
		String::from_utf8(crab).unwrap_or_default()
	);

	let server = tiny_http::Server::http("0.0.0.0:9975").unwrap();
	println!("Now listening on port 9975");

	for rq in server.incoming_requests() {
		let response = tiny_http::Response::from_string(text.to_string())
			.with_status_code(200)
			.with_header(tiny_http::Header {
				field: "Content-Type".parse().unwrap(),
				value: AsciiString::from_ascii("text/plain; charset=utf8").unwrap(),
			});
		let _ = rq.respond(response);
	}
}
