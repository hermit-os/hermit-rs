#[cfg(target_os = "hermit")]
use hermit_sys as _;

use ascii::AsciiString;
use std::fs;
use std::path::Path;

extern crate ascii;
extern crate tiny_http;

#[cfg(target_os = "hermit")]
const ROOT_DIR: &str = "/root";
#[cfg(not(target_os = "hermit"))]
const ROOT_DIR: &str = "/tmp/data";

fn get_content_type(path: &Path) -> &'static str {
	let extension = match path.extension() {
		None => return "text/plain",
		Some(e) => e,
	};

	match extension.to_str().unwrap() {
		"gif" => "image/gif",
		"jpg" => "image/jpeg",
		"jpeg" => "image/jpeg",
		"png" => "image/png",
		"pdf" => "application/pdf",
		"htm" => "text/html; charset=utf8",
		"html" => "text/html; charset=utf8",
		"txt" => "text/plain; charset=utf8",
		"css" => "text/css; charset=utf8",
		_ => "text/plain; charset=utf8",
	}
}

fn main() {
	let root_path = Path::new(ROOT_DIR);
	assert!(root_path.is_dir());

	#[cfg(not(target_os = "hermit"))]
	let server = tiny_http::Server::http("0.0.0.0:9000").unwrap();

	#[cfg(target_os = "hermit")]
	let server = tiny_http::Server::http("0.0.0.0:8000").unwrap();

	let port = server.server_addr().to_ip().unwrap().port();
	println!("Now listening on port {}", port);

	loop {
		let rq = match server.recv() {
			Ok(rq) => rq,
			Err(_) => break,
		};

		println!("{:?}", rq);

		let url = rq.url().to_string();
		let path = root_path.join(Path::new(&url[1..]));
		println!("Path: {}", path.display());
		let file = fs::File::open(&path);

		if file.is_ok() {
			let response = tiny_http::Response::from_file(file.unwrap());

			let response = response.with_header(tiny_http::Header {
				field: "Content-Type".parse().unwrap(),
				value: AsciiString::from_ascii(get_content_type(&path)).unwrap(),
			});

			let _ = rq.respond(response);
		} else {
			let rep = tiny_http::Response::new_empty(tiny_http::StatusCode(404));
			let _ = rq.respond(rep);
		}
	}
}
