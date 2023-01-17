/// Example is derived from tiny-http example
/// https://github.com/tiny-http/tiny-http/blob/master/examples/hello-world.rs
use chrono::{DateTime, Utc};
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

	// In the CI httpd should only answer one request

	#[cfg(not(feature = "ci"))]
	for request in server.incoming_requests() {
		println!(
			"received request! method: {:?}, url: {:?}, headers: {:?}",
			request.method(),
			request.url(),
			request.headers()
		);

		let text = format!(
			"Hello from RustyHermit {}!\nThe current UTC time is {}!\n",
			String::from_utf8(crab).unwrap_or_default(),
			now.format("%Y-%m-%d %H:%M:%S")
		);
		let response = tiny_http::Response::from_string(text);
		request.respond(response).expect("Responded");
	}

	#[cfg(feature = "ci")]
	if let Some(request) = server.incoming_requests().next() {
		println!(
			"received request! method: {:?}, url: {:?}, headers: {:?}",
			request.method(),
			request.url(),
			request.headers()
		);

		let text = format!(
			"Hello from RustyHermit {}!\nThe current UTC time is {}!\n",
			String::from_utf8(crab).unwrap_or_default(),
			now.format("%Y-%m-%d %H:%M:%S")
		);
		let response = tiny_http::Response::from_string(text);
		request.respond(response).expect("Responded");
	}
}
