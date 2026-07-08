mod benchmark;
mod bw;
mod config;
mod connection;
mod latency;
mod print_utils;
mod threading;

use benchmark::Benchmark;
use bw::{BwClient, BwServer};
use clap::{Parser, Subcommand};
use config::Config;
#[cfg(target_os = "hermit")]
use hermit as _;
use latency::{LatencyClient, LatencyServer};

#[derive(Clone, Copy)]
pub(crate) enum Protocol {
	Tcp,
	Udp,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
	#[command(subcommand)]
	command: Command,
}

#[derive(Subcommand)]
enum Command {
	Bw {
		#[command(subcommand)]
		protocol: ProtocolCommand,
	},
	Latency {
		#[command(subcommand)]
		protocol: ProtocolCommand,
	},
}

#[derive(Subcommand)]
enum ProtocolCommand {
	Tcp {
		#[command(subcommand)]
		role: Role,
	},
	Udp {
		#[command(subcommand)]
		role: Role,
	},
}

#[derive(Subcommand)]
enum Role {
	Server(Config),
	Client(Config),
}

fn main() {
	match Cli::parse().command {
		Command::Bw { protocol } => dispatch::<BwServer, BwClient>(protocol),
		Command::Latency { protocol } => dispatch::<LatencyServer, LatencyClient>(protocol),
	}
}

fn dispatch<Server: Benchmark, Client: Benchmark>(protocol_cmd: ProtocolCommand) {
	match protocol_cmd {
		ProtocolCommand::Tcp { role } => match role {
			Role::Server(args) => Server::execute(Protocol::Tcp, args),
			Role::Client(args) => Client::execute(Protocol::Tcp, args),
		},
		ProtocolCommand::Udp { role } => match role {
			Role::Server(args) => Server::execute(Protocol::Udp, args),
			Role::Client(args) => Client::execute(Protocol::Udp, args),
		},
	}
}
