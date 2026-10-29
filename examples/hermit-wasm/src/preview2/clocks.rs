use std::time::{Duration, SystemTime, UNIX_EPOCH};

use wasmtime::component::Resource;
use wasmtime_wasi_io::poll::{DynPollable, subscribe};

use crate::preview2::WasiCtx;
use crate::preview2::bindings::clocks::monotonic_clock::{self, Instant};
use crate::preview2::bindings::clocks::wall_clock::{self, Datetime};
use crate::preview2::io::Deadline;

impl WasiCtx {
	fn subscribe_deadline(
		&mut self,
		deadline: std::time::Instant,
	) -> wasmtime::Result<Resource<DynPollable>> {
		let deadline = self.table.push(Deadline(deadline))?;
		subscribe(&mut self.table, deadline)
	}
}

impl monotonic_clock::Host for WasiCtx {
	fn now(&mut self) -> wasmtime::Result<Instant> {
		Ok(self.start.elapsed().as_nanos().try_into()?)
	}

	fn resolution(&mut self) -> wasmtime::Result<monotonic_clock::Duration> {
		Ok(1)
	}

	fn subscribe_instant(&mut self, when: Instant) -> wasmtime::Result<Resource<DynPollable>> {
		let deadline = self.start + Duration::from_nanos(when);
		self.subscribe_deadline(deadline)
	}

	fn subscribe_duration(
		&mut self,
		when: monotonic_clock::Duration,
	) -> wasmtime::Result<Resource<DynPollable>> {
		let deadline = std::time::Instant::now() + Duration::from_nanos(when);
		self.subscribe_deadline(deadline)
	}
}

impl wall_clock::Host for WasiCtx {
	fn now(&mut self) -> wasmtime::Result<Datetime> {
		let now = SystemTime::now().duration_since(UNIX_EPOCH)?;
		Ok(Datetime {
			seconds: now.as_secs(),
			nanoseconds: now.subsec_nanos(),
		})
	}

	fn resolution(&mut self) -> wasmtime::Result<Datetime> {
		Ok(Datetime {
			seconds: 0,
			nanoseconds: 1,
		})
	}
}
