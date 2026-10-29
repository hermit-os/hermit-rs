use std::ffi::OsString;
use std::fmt;
use std::time::Instant;

use log::debug;
use wasmtime::component::types::ComponentItem;
use wasmtime::component::{Component, HasSelf, Linker, ResourceTable, ResourceType};
use wasmtime::{Engine, Result, Store, bail};
use wasmtime_wasi_io::poll::DynPollable;
use wasmtime_wasi_io::streams::{DynInputStream, DynOutputStream};

mod bindings;
mod cli;
mod clocks;
mod filesystem;
mod io;
mod random;

/// Error used to terminate a component with an exit code.
#[derive(Debug)]
pub struct I32Exit(pub i32);

impl fmt::Display for I32Exit {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "exit with code {}", self.0)
	}
}

impl std::error::Error for I32Exit {}

pub(crate) struct WasiCtx {
	args: Vec<String>,
	env: Vec<(String, String)>,
	start: Instant,
	table: ResourceTable,
}

fn add_to_linker(linker: &mut Linker<WasiCtx>) -> Result<()> {
	use bindings::{cli, clocks, filesystem, io, random};

	fn table(ctx: &mut WasiCtx) -> &mut ResourceTable {
		&mut ctx.table
	}

	io::error::add_to_linker::<_, HasSelf<ResourceTable>>(linker, table)?;
	io::poll::add_to_linker::<_, HasSelf<ResourceTable>>(linker, table)?;
	io::streams::add_to_linker::<_, HasSelf<ResourceTable>>(linker, table)?;

	cli::environment::add_to_linker::<_, HasSelf<_>>(linker, |ctx| ctx)?;
	cli::exit::add_to_linker::<_, HasSelf<_>>(linker, |ctx| ctx)?;
	cli::stdin::add_to_linker::<_, HasSelf<_>>(linker, |ctx| ctx)?;
	cli::stdout::add_to_linker::<_, HasSelf<_>>(linker, |ctx| ctx)?;
	cli::stderr::add_to_linker::<_, HasSelf<_>>(linker, |ctx| ctx)?;
	cli::terminal_input::add_to_linker::<_, HasSelf<_>>(linker, |ctx| ctx)?;
	cli::terminal_output::add_to_linker::<_, HasSelf<_>>(linker, |ctx| ctx)?;
	cli::terminal_stdin::add_to_linker::<_, HasSelf<_>>(linker, |ctx| ctx)?;
	cli::terminal_stdout::add_to_linker::<_, HasSelf<_>>(linker, |ctx| ctx)?;
	cli::terminal_stderr::add_to_linker::<_, HasSelf<_>>(linker, |ctx| ctx)?;

	clocks::monotonic_clock::add_to_linker::<_, HasSelf<_>>(linker, |ctx| ctx)?;
	clocks::wall_clock::add_to_linker::<_, HasSelf<_>>(linker, |ctx| ctx)?;

	filesystem::types::add_to_linker::<_, HasSelf<_>>(linker, |ctx| ctx)?;
	filesystem::preopens::add_to_linker::<_, HasSelf<_>>(linker, |ctx| ctx)?;

	random::random::add_to_linker::<_, HasSelf<_>>(linker, |ctx| ctx)?;
	random::insecure::add_to_linker::<_, HasSelf<_>>(linker, |ctx| ctx)?;
	random::insecure_seed::add_to_linker::<_, HasSelf<_>>(linker, |ctx| ctx)?;

	Ok(())
}

const IMPLEMENTED: &[&str] = &[
	"wasi:io/error",
	"wasi:io/poll",
	"wasi:io/streams",
	"wasi:cli/environment",
	"wasi:cli/exit",
	"wasi:cli/stdin",
	"wasi:cli/stdout",
	"wasi:cli/stderr",
	"wasi:cli/terminal-input",
	"wasi:cli/terminal-output",
	"wasi:cli/terminal-stdin",
	"wasi:cli/terminal-stdout",
	"wasi:cli/terminal-stderr",
	"wasi:clocks/monotonic-clock",
	"wasi:clocks/wall-clock",
	"wasi:filesystem/types",
	"wasi:filesystem/preopens",
	"wasi:random/random",
	"wasi:random/insecure",
	"wasi:random/insecure-seed",
];

/// Defines all functions of not yet supported interfaces as traps.
///
/// In contrast to `Linker::define_unknown_imports_as_traps`, resources
/// of `wasi:io` keep their host types, so that interfaces using them
/// still type-check.
fn stub_unsupported_imports(
	linker: &mut Linker<WasiCtx>,
	engine: &Engine,
	component: &Component,
) -> Result<()> {
	let component_type = component.component_type();
	for (name, import) in component_type.imports(engine) {
		let interface = name.split('@').next().unwrap_or(name);
		if IMPLEMENTED.contains(&interface) {
			continue;
		}
		let ComponentItem::ComponentInstance(instance) = import.ty else {
			bail!("unsupported import `{name}`");
		};

		let mut linker_instance = linker.instance(name)?;
		for (export_name, export) in instance.exports(engine) {
			match export.ty {
				ComponentItem::ComponentFunc(_) => {
					let qualified_name = format!("{name}#{export_name}");
					linker_instance.func_new(export_name, move |_, _, _, _| {
						bail!("unsupported import `{qualified_name}`")
					})?;
				}
				ComponentItem::Resource(_) => {
					let ty = match export_name {
						"pollable" => ResourceType::host::<DynPollable>(),
						"input-stream" => ResourceType::host::<DynInputStream>(),
						"output-stream" => ResourceType::host::<DynOutputStream>(),
						"error" => ResourceType::host::<wasmtime::Error>(),
						_ => ResourceType::host::<()>(),
					};
					linker_instance.resource(export_name, ty, |_, _| Ok(()))?;
				}
				_ => {}
			}
		}
	}

	Ok(())
}

/// Runs a component implementing the `wasi:cli/command` world.
///
/// Returns the exit code of the component.
pub fn run(component_bytes: &[u8], config: &wasmtime::Config, args: &[OsString]) -> Result<i32> {
	let engine = Engine::new(config)?;

	let now = Instant::now();
	let component = Component::new(&engine, component_bytes)?;
	debug!(
		"Time to create component: {} msec",
		now.elapsed().as_millis()
	);

	let mut linker = Linker::new(&engine);
	add_to_linker(&mut linker)?;
	stub_unsupported_imports(&mut linker, &engine, &component)?;

	let ctx = WasiCtx {
		args: args
			.iter()
			.map(|arg| arg.to_string_lossy().into_owned())
			.collect(),
		env: std::env::vars().collect(),
		start: Instant::now(),
		table: ResourceTable::new(),
	};
	let mut store = Store::new(&engine, ctx);

	let command = bindings::Command::instantiate(&mut store, &component, &linker)?;
	match command.wasi_cli_run().call_run(&mut store) {
		Ok(Ok(())) => Ok(0),
		Ok(Err(())) => Ok(1),
		Err(e) => match e.downcast_ref::<I32Exit>() {
			Some(exit) => Ok(exit.0),
			None => Err(e),
		},
	}
}
