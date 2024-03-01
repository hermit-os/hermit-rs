#[cfg(target_os = "hermit")]
use hermit as _;

use std::fs;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::Path;

use http::{Method, Request, Response, StatusCode};
use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

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

// An async function that consumes a request, does nothing with it and returns a
// response.
async fn hello(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, hyper::Error> {
	let response = match (req.method(), req.uri().path()) {
		// Help route.
		(&Method::GET, path) => {
			let file_path = if path == "/" {
				ROOT_DIR.to_string() + "/index.html"
			} else {
				ROOT_DIR.to_string() + req.uri().path()
			};
			let file_path = Path::new(&file_path);

			if let Ok(data) = fs::read(&file_path) {
				Response::builder()
					.header("Content-Type", get_content_type(file_path))
					.header("content-length", data.len())
					.body(Full::from(data))
					.unwrap()
			} else {
				Response::builder()
					.status(StatusCode::NOT_FOUND)
					.body(Full::default())
					.unwrap()
			}
		}
		// Echo service route.
		(&Method::POST, "/echo") => {
			let data = req.into_body().collect().await?.to_bytes();
			Response::builder()
				.header("Content-Type", "text/plain; charset=utf8")
				.header("content-length", data.len())
				.body(Full::from(data))
				.unwrap()
		}
		// Catch-all 404.
		_ => Response::builder()
			.status(StatusCode::NOT_FOUND)
			.body(Full::default())
			.unwrap(),
	};

	Ok(response)
}

#[tokio::main(flavor = "current_thread")]
pub async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
	env_logger::Builder::new().parse_filters("debug").init();

	// This address is localhost
	let addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), 9975);

	// Bind to the port and listen for incoming TCP connections
	let listener = TcpListener::bind(addr).await?;
	println!("Listening on http://{}", addr);
	loop {
		// When an incoming TCP connection is received grab a TCP stream for
		// client<->server communication.
		//
		// Note, this is a .await point, this loop will loop forever but is not a busy loop. The
		// .await point allows the Tokio runtime to pull the task off of the thread until the task
		// has work to do. In this case, a connection arrives on the port we are listening on and
		// the task is woken up, at which point the task is then put back on a thread, and is
		// driven forward by the runtime, eventually yielding a TCP stream.
		let (tcp, _) = listener.accept().await?;
		// Use an adapter to access something implementing `tokio::io` traits as if they implement
		// `hyper::rt` IO traits.
		let io = TokioIo::new(tcp);

		// Spin up a new task in Tokio so we can continue to listen for new TCP connection on the
		// current task without waiting for the processing of the HTTP1 connection we just received
		// to finish
		tokio::task::spawn(async move {
			// Handle the connection from the client using HTTP1 and pass any
			// HTTP requests received on that connection to the `hello` function
			if let Err(err) = http1::Builder::new()
				.serve_connection(io, service_fn(hello))
				.await
			{
				println!("Error serving connection: {:?}", err);
			}
		});
	}
}
