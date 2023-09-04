#[cfg(target_os = "hermit")]
use hermit as _;

fn handle_request(request: tiny_http::Request) {
	eprintln!("{request:?}");

	let now_utc = time::OffsetDateTime::now_utc();
	let text = format!("Hello from Hermit! 🦀\nThe current date and time in UTC is {now_utc}.");
	let response = tiny_http::Response::from_string(text);

	request.respond(response).unwrap();
}

fn main() {
	let server = tiny_http::Server::http("0.0.0.0:9975").unwrap();
	eprintln!("Now listening on port 9975");

	for request in server.incoming_requests() {
		handle_request(request);

		if cfg!(feature = "ci") {
			break;
		}
	}
}
