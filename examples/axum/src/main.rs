use axum::routing::get;
use axum::Router;
#[cfg(target_os = "hermit")]
use hermit as _;
use tokio::{io, net};

#[tokio::main(flavor = "current_thread")]
async fn main() -> io::Result<()> {
	let app = Router::new().route("/", get(root));

	let listener = net::TcpListener::bind("0.0.0.0:9975").await?;
	axum::serve(listener, app).await?;

	Ok(())
}

async fn root() -> &'static str {
	"Hello, World!"
}
