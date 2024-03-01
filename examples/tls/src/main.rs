//! Simple HTTPS echo service based on hyper_util and rustls
//!
//! The example is derived from `<https://github.com/rustls/hyper-rustls>`
//! Certificate and private key are hardcoded to sample files.
//! hyper will automatically use HTTP/2 if a client starts talking HTTP/2,
//! otherwise HTTP/1.1 will be used.
//!
//! Test the server with follow command:
//! curl --verbose --insecure -d "Hello World" -X POST https://127.0.0.1:9975/echo

#[cfg(target_os = "hermit")]
use hermit as _;

use std::io;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;

use http::{Method, Request, Response, StatusCode};
use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::service::service_fn;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder;
use pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

#[tokio::main(flavor = "current_thread")]
pub async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
	env_logger::Builder::new().parse_filters("debug").init();

	// see readme of rustls-rustcrypto
	println!("USE THIS AT YOUR OWN RISK!");
	println!("See https://github.com/RustCrypto/rustls-rustcrypto for details!");

	let addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), 9975);

	// Load public certificate.
	let certs = load_certs()?;
	// Load private key.
	let key = load_private_key()?;

	println!("Starting to serve on https://{}", addr);

	// Create a TCP listener via tokio.
	let incoming = TcpListener::bind(addr).await?;

	// Build TLS configuration.
	let mut server_config =
		ServerConfig::builder_with_provider(Arc::new(rustls_rustcrypto::provider()))
			.with_safe_default_protocol_versions()?
			.with_no_client_auth()
			.with_single_cert(certs, key)?;
	server_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec(), b"http/1.0".to_vec()];
	let tls_acceptor = TlsAcceptor::from(Arc::new(server_config));

	let service = service_fn(echo);

	loop {
		let (tcp_stream, _remote_addr) = incoming.accept().await?;

		let tls_acceptor = tls_acceptor.clone();
		tokio::spawn(async move {
			let tls_stream = match tls_acceptor.accept(tcp_stream).await {
				Ok(tls_stream) => tls_stream,
				Err(err) => {
					eprintln!("failed to perform tls handshake: {err:#}");
					return;
				}
			};
			if let Err(_err) = Builder::new(TokioExecutor::new())
				.serve_connection(TokioIo::new(tls_stream), service)
				.await
			{
				//eprintln!("failed to serve connection: {err:#}");
			}
		});
	}
}

// Custom echo service, handling two different routes and a
// catch-all 404 responder.
async fn echo(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, hyper::Error> {
	let response = match (req.method(), req.uri().path()) {
		// Help route.
		(&Method::GET, "/") => {
			let response = "Hello from HermitOS! 🦀\nTry POST /echo";
			Response::builder()
				.header("Content-Type", "text/plain; charset=utf8")
				.header("content-length", response.len())
				.body(Full::from(response.as_bytes()))
				.unwrap()
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

// Load public certificate from file.
fn load_certs() -> io::Result<Vec<CertificateDer<'static>>> {
	// using an in-memory file just to avoid FUSE configuration
	static CERTS: &[u8] = include_bytes!("./sample.pem");
	let buf = io::Cursor::new(CERTS);
	let mut reader = io::BufReader::new(buf);

	// Load and return certificate.
	rustls_pemfile::certs(&mut reader).collect()
}

// Load private key from file.
fn load_private_key() -> io::Result<PrivateKeyDer<'static>> {
	// using an in-memory file just to avoid FUSE configuration
	static KEY: &[u8] = include_bytes!("./sample.rsa");
	let buf = io::Cursor::new(KEY);
	let mut reader = io::BufReader::new(buf);

	// Load and return a single private key.
	rustls_pemfile::private_key(&mut reader).map(|key| key.unwrap())
}
