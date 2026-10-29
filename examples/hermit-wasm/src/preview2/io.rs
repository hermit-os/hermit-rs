use std::cell::Cell;
use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::thread::{self, Thread};
use std::time::Instant;

use wasmtime::component::{Resource, ResourceTable};
use wasmtime_wasi_io::bindings::wasi::io::{poll as async_poll, streams as async_streams};
use wasmtime_wasi_io::poll::DynPollable;
use wasmtime_wasi_io::streams::{DynInputStream, DynOutputStream, StreamResult};

use crate::preview2::bindings::io::{poll, streams};

thread_local! {
	static NEXT_DEADLINE: Cell<Option<Instant>> = const { Cell::new(None) };
}

struct ThreadWaker(Thread);

impl Wake for ThreadWaker {
	fn wake(self: Arc<Self>) {
		self.0.unpark();
	}
}

/// Runs `future` to completion on the current thread.
///
/// Pending futures either register a deadline via [`Deadline`] or are
/// woken through the waker.
pub(crate) fn block_on<F: Future>(future: F) -> F::Output {
	let mut future = pin!(future);
	let waker = Waker::from(Arc::new(ThreadWaker(thread::current())));
	let mut cx = Context::from_waker(&waker);

	loop {
		NEXT_DEADLINE.set(None);
		if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
			return output;
		}
		match NEXT_DEADLINE.get() {
			Some(deadline) => {
				thread::park_timeout(deadline.saturating_duration_since(Instant::now()))
			}
			None => thread::park(),
		}
	}
}

/// Pollable which becomes ready at a fixed point in time.
pub(crate) struct Deadline(pub Instant);

impl Future for &Deadline {
	type Output = ();

	fn poll(self: std::pin::Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
		if Instant::now() >= self.0 {
			return Poll::Ready(());
		}
		NEXT_DEADLINE.set(Some(match NEXT_DEADLINE.get() {
			Some(next) => next.min(self.0),
			None => self.0,
		}));
		Poll::Pending
	}
}

#[wasmtime_wasi_io::async_trait]
impl wasmtime_wasi_io::poll::Pollable for Deadline {
	async fn ready(&mut self) {
		(&*self).await
	}
}

impl From<async_streams::StreamError> for streams::StreamError {
	fn from(other: async_streams::StreamError) -> Self {
		match other {
			async_streams::StreamError::LastOperationFailed(e) => Self::LastOperationFailed(e),
			async_streams::StreamError::Closed => Self::Closed,
		}
	}
}

impl poll::Host for ResourceTable {
	fn poll(&mut self, pollables: Vec<Resource<DynPollable>>) -> wasmtime::Result<Vec<u32>> {
		block_on(async_poll::Host::poll(self, pollables))
	}
}

impl poll::HostPollable for ResourceTable {
	fn ready(&mut self, pollable: Resource<DynPollable>) -> wasmtime::Result<bool> {
		block_on(async_poll::HostPollable::ready(self, pollable))
	}

	fn block(&mut self, pollable: Resource<DynPollable>) -> wasmtime::Result<()> {
		block_on(async_poll::HostPollable::block(self, pollable))
	}

	fn drop(&mut self, pollable: Resource<DynPollable>) -> wasmtime::Result<()> {
		async_poll::HostPollable::drop(self, pollable)
	}
}

impl streams::Host for ResourceTable {
	fn convert_stream_error(
		&mut self,
		err: wasmtime_wasi_io::streams::StreamError,
	) -> wasmtime::Result<streams::StreamError> {
		Ok(async_streams::Host::convert_stream_error(self, err)?.into())
	}
}

impl streams::HostOutputStream for ResourceTable {
	fn drop(&mut self, stream: Resource<DynOutputStream>) -> wasmtime::Result<()> {
		block_on(async_streams::HostOutputStream::drop(self, stream))
	}

	fn check_write(&mut self, stream: Resource<DynOutputStream>) -> StreamResult<u64> {
		async_streams::HostOutputStream::check_write(self, stream)
	}

	fn write(&mut self, stream: Resource<DynOutputStream>, bytes: Vec<u8>) -> StreamResult<()> {
		async_streams::HostOutputStream::write(self, stream, bytes)
	}

	fn blocking_write_and_flush(
		&mut self,
		stream: Resource<DynOutputStream>,
		bytes: Vec<u8>,
	) -> StreamResult<()> {
		block_on(async_streams::HostOutputStream::blocking_write_and_flush(
			self, stream, bytes,
		))
	}

	fn blocking_write_zeroes_and_flush(
		&mut self,
		stream: Resource<DynOutputStream>,
		len: u64,
	) -> StreamResult<()> {
		block_on(
			async_streams::HostOutputStream::blocking_write_zeroes_and_flush(self, stream, len),
		)
	}

	fn subscribe(
		&mut self,
		stream: Resource<DynOutputStream>,
	) -> wasmtime::Result<Resource<DynPollable>> {
		async_streams::HostOutputStream::subscribe(self, stream)
	}

	fn write_zeroes(&mut self, stream: Resource<DynOutputStream>, len: u64) -> StreamResult<()> {
		async_streams::HostOutputStream::write_zeroes(self, stream, len)
	}

	fn flush(&mut self, stream: Resource<DynOutputStream>) -> StreamResult<()> {
		async_streams::HostOutputStream::flush(self, stream)
	}

	fn blocking_flush(&mut self, stream: Resource<DynOutputStream>) -> StreamResult<()> {
		block_on(async_streams::HostOutputStream::blocking_flush(
			self, stream,
		))
	}

	fn splice(
		&mut self,
		dst: Resource<DynOutputStream>,
		src: Resource<DynInputStream>,
		len: u64,
	) -> StreamResult<u64> {
		async_streams::HostOutputStream::splice(self, dst, src, len)
	}

	fn blocking_splice(
		&mut self,
		dst: Resource<DynOutputStream>,
		src: Resource<DynInputStream>,
		len: u64,
	) -> StreamResult<u64> {
		block_on(async_streams::HostOutputStream::blocking_splice(
			self, dst, src, len,
		))
	}
}

impl streams::HostInputStream for ResourceTable {
	fn drop(&mut self, stream: Resource<DynInputStream>) -> wasmtime::Result<()> {
		block_on(async_streams::HostInputStream::drop(self, stream))
	}

	fn read(&mut self, stream: Resource<DynInputStream>, len: u64) -> StreamResult<Vec<u8>> {
		async_streams::HostInputStream::read(self, stream, len)
	}

	fn blocking_read(
		&mut self,
		stream: Resource<DynInputStream>,
		len: u64,
	) -> StreamResult<Vec<u8>> {
		block_on(async_streams::HostInputStream::blocking_read(
			self, stream, len,
		))
	}

	fn skip(&mut self, stream: Resource<DynInputStream>, len: u64) -> StreamResult<u64> {
		async_streams::HostInputStream::skip(self, stream, len)
	}

	fn blocking_skip(&mut self, stream: Resource<DynInputStream>, len: u64) -> StreamResult<u64> {
		block_on(async_streams::HostInputStream::blocking_skip(
			self, stream, len,
		))
	}

	fn subscribe(
		&mut self,
		stream: Resource<DynInputStream>,
	) -> wasmtime::Result<Resource<DynPollable>> {
		async_streams::HostInputStream::subscribe(self, stream)
	}
}
