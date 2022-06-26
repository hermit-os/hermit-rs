/// Example is derived from tiny-http example
/// https://github.com/tiny-http/tiny-http/blob/master/examples/hello-world.rs
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

	for request in server.incoming_requests() {
		println!(
			"received request! method: {:?}, url: {:?}, headers: {:?}",
			request.method(),
			request.url(),
			request.headers()
		);

		let response = tiny_http::Response::from_string(text.clone());
		request.respond(response).expect("Responded");

		// In the CI httpd should only answer one request
		#[cfg(not(feature = "ci"))]
		return;
	}
}
