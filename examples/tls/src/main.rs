//! Simple HTTPS echo service based on hyper_util and rustls
//!
//! The example is derived from https://github.com/rustls/hyper-rustls
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
use std::vec::Vec;

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

fn main() {
	env_logger::Builder::new().parse_filters("debug").init();

	// see readme of rustls-rustcrypto
	println!("USE THIS AT YOUR OWN RISK!");
	println!("See https://github.com/RustCrypto/rustls-rustcrypto for details!");

	// Serve an echo service over HTTPS, with proper error handling.
	if let Err(e) = run_server() {
		eprintln!("FAILED: {}", e);
		std::process::exit(1);
	}
}

fn error(err: String) -> io::Error {
	io::Error::new(io::ErrorKind::Other, err)
}

#[tokio::main(flavor = "current_thread")]
async fn run_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
			.with_single_cert(certs, key)
			.map_err(|e| error(e.to_string()))?;
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
	let mut response = Response::new(Full::default());
	match (req.method(), req.uri().path()) {
		// Help route.
		(&Method::GET, "/") => {
			*response.body_mut() = Full::from("Try POST /echo\n");
		}
		// Echo service route.
		(&Method::POST, "/echo") => {
			*response.body_mut() = Full::from(req.into_body().collect().await?.to_bytes());
		}
		// Catch-all 404.
		_ => {
			*response.status_mut() = StatusCode::NOT_FOUND;
		}
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
