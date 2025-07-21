use std::io;

#[cfg(target_os = "hermit")]
use hermit as _;

fn main() -> io::Result<()> {
	io::copy(&mut io::stdin(), &mut io::stdout())?;
	Ok(())
}
