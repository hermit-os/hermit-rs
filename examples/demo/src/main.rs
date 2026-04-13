use std::{env, f64, hint, io};

#[cfg(target_os = "hermit")]
use hermit as _;

mod fs;
mod laplace;
mod mandelbrot;
mod matmul;
mod pi;
mod thread;

fn main() -> io::Result<()> {
	hello();
	print_env();
	arithmetic();
	thread::sleep();
	thread::spawn()?;
	fs::fs()?;
	pi::pi();
	//matmul::matmul();
	//laplace::laplace();
	mandelbrot::mandelbrot();
	Ok(())
}

pub fn hello() {
	eprintln!();
	eprintln!("Hello, Hermit! 🦀");
	eprintln!("Hello, world!");
	eprintln!("Привет, мир!");
	eprintln!("こんにちは世界！");
	eprintln!("你好世界！");
	eprintln!("สวัสดีชาวโลก!");
	eprintln!("Chào thế giới!");
}

pub fn print_env() {
	eprintln!();
	eprintln!("Arguments:");
	for argument in env::args() {
		eprintln!("{argument}");
	}

	eprintln!();
	eprintln!("Environment variables:");
	for (key, value) in env::vars() {
		eprintln!("{key}: {value}");
	}
}

pub fn arithmetic() {
	eprintln!();

	let x = hint::black_box(f64::consts::PI) * 2.0;
	let y: f64 = hint::black_box(x).exp();
	let z: f64 = hint::black_box(y).ln();

	eprintln!("x = {x}");
	eprintln!("e^x = {y}");
	eprintln!("ln(e^x) = {z}");
}
