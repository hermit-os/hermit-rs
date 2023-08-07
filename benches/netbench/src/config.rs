use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(next_line_help = true)]
pub struct Config {
	#[arg(
		long,
		short,
		default_value = "127.0.0.1",
		help = "IP4 address to connect to"
	)]
	pub address: String,
	#[arg(
		long,
		short,
		default_value_t = 7878,
		help = "port to connect to, like port 7878 if you want to connect to 127.0.0.1:7878"
	)]
	pub port: u16,
	#[arg(
		long = "bytes",
		short = 'k',
		default_value_t = 1,
		help = "number of bytes to transfer every round"
	)]
	pub n_bytes: usize,
	#[arg(
		long = "rounds",
		short = 'r',
		default_value_t = 100000,
		help = "number of rounds of transfer to perform"
	)]
	pub n_rounds: usize,
	#[arg(
		long = "nodelay",
		short = 'd',
		help = "sets TCP in no-delay mode"
	)]
	pub no_delay: bool,
	#[arg(
		long = "nonblocking",
		short = 'b',
		help = "sets TCP in non-blocking mode"
	)]
	pub non_blocking: bool,
	#[arg(
        long = "thread",
        short = 't',
        default_value_t = -1,
        help = "id of process to pin thread to, -1 for no pinning"
    )]
	pub p_id: i8,
}

impl Config {
	pub fn address_and_port(&self) -> String {
		format!("{}:{}", &self.address, &self.port)
	}
}
