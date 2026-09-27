//! Benchmarks for the file system mounted at `/root`.
//!
//! Everything goes through `std::fs`, so a measurement covers the whole stack:
//! the VFS, the FAT layer, the sector cache and the block driver underneath.
//!
//! Run it with a FAT-formatted image attached, for example
//!
//! ```text
//! qemu-system-aarch64 ... \
//!   -drive file=disk.img,format=raw,if=none,id=disk0 \
//!   -device virtio-blk-pci,drive=disk0,disable-legacy=on
//! ```

use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

#[cfg(target_os = "hermit")]
use hermit as _;
use hermit_bench_output::log_benchmark_data_with_group;

/// Where the block device is mounted.
const MOUNT: &str = "/root";

/// Size of the file the read benchmarks work on.
const FILE_SIZE: usize = 1024 * 1024;

/// Buffer sizes to read with.
///
/// The spread matters: a positioned read walks the FAT chain from the start of
/// the file, so the cost per byte grows as the buffer shrinks.
const READ_BUFFERS: [usize; 3] = [512, 4096, 65536];

/// Buffer sizes to write with.
///
/// Deliberately coarser than [`READ_BUFFERS`]. A write rewrites the file from
/// its beginning, so halving the buffer roughly quadruples the work; 512-byte
/// writes of a file this size would dominate the whole run.
const WRITE_BUFFERS: [usize; 2] = [65536, 16384];

/// Size of the file the write benchmarks produce.
const WRITE_SIZE: usize = 256 * 1024;

/// Number of operations for the metadata and random-access benchmarks.
const OPS: usize = 200;

/// Threads the concurrent benchmarks use, unless the system reports fewer
/// cores.
const THREADS: usize = 4;

/// Size of the file each thread of the concurrent benchmarks owns.
const THREAD_FILE_SIZE: usize = 256 * 1024;

/// Buffer the concurrent benchmarks transfer with.
const THREAD_BUFFER: usize = 65536;

fn path(name: &str) -> String {
	format!("{MOUNT}/{name}")
}

/// Byte expected at `pos` of the generated files.
fn pattern(pos: usize) -> u8 {
	(pos % 251) as u8
}

fn throughput(bytes: usize, elapsed: Duration) -> f64 {
	let secs = elapsed.as_secs_f64();
	if secs <= 0.0 {
		return 0.0;
	}

	bytes as f64 / secs / (1024.0 * 1024.0)
}

fn micros_per_op(ops: usize, elapsed: Duration) -> f64 {
	elapsed.as_secs_f64() * 1_000_000.0 / ops as f64
}

/// Creates the file the read benchmarks use, if it is not there yet.
fn prepare(name: &str, size: usize) -> std::io::Result<()> {
	// A leftover from an aborted run is not trusted, not even when the size
	// matches: it may hold a half-written cluster chain.
	let path = path(name);
	let _ = fs::remove_file(&path);

	let mut file = File::create(&path)?;
	let chunk: Vec<u8> = (0..65536).map(pattern).collect();
	let mut written = 0;
	while written < size {
		let n = usize::min(chunk.len(), size - written);
		// The pattern repeats every 251 bytes and the chunk length is a
		// multiple of neither, so the offset has to be carried explicitly.
		let piece: Vec<u8> = (written..written + n).map(pattern).collect();
		file.write_all(&piece)?;
		written += n;
	}
	file.flush()?;

	Ok(())
}

/// Reads the whole file with `buffer` sized reads and verifies the contents.
fn bench_sequential_read(name: &str, buffer: usize) -> std::io::Result<()> {
	let mut file = File::open(path(name))?;
	let mut buf = vec![0u8; buffer];
	let mut total = 0usize;

	let start = Instant::now();
	loop {
		let n = file.read(&mut buf)?;
		if n == 0 {
			break;
		}
		total += n;
	}
	let elapsed = start.elapsed();

	assert_eq!(total, FILE_SIZE, "short read with a {buffer} byte buffer");
	log_benchmark_data_with_group(
		&format!("sequential read, {buffer} byte buffer"),
		"MiB/s",
		throughput(total, elapsed),
		"file system read",
	);

	Ok(())
}

/// Writes a fresh file with `buffer` sized writes.
fn bench_sequential_write(name: &str, buffer: usize) -> std::io::Result<()> {
	let path = path(name);
	let _ = fs::remove_file(&path);

	let chunk: Vec<u8> = (0..buffer).map(pattern).collect();
	let mut file = File::create(&path)?;

	let start = Instant::now();
	let mut written = 0usize;
	while written < WRITE_SIZE {
		let n = usize::min(buffer, WRITE_SIZE - written);
		file.write_all(&chunk[..n])?;
		written += n;
	}
	file.flush()?;
	let elapsed = start.elapsed();

	log_benchmark_data_with_group(
		&format!("sequential write, {buffer} byte buffer"),
		"MiB/s",
		throughput(written, elapsed),
		"file system write",
	);

	fs::remove_file(&path)?;

	Ok(())
}

/// Seeks to scattered offsets and reads a sector at each.
///
/// This isolates the positioning cost from the data transfer: every read moves
/// the same 512 bytes, only the distance the FAT chain has to be walked varies.
fn bench_random_read(name: &str) -> std::io::Result<()> {
	let mut file = File::open(path(name))?;
	let mut buf = [0u8; 512];

	// Cheap deterministic offsets — a real RNG would add a dependency for no
	// benefit, and reproducibility is worth more here than statistical purity.
	let mut state = 0x2545_f491_4f6c_dd1du64;
	let mut offsets = Vec::with_capacity(OPS);
	for _ in 0..OPS {
		state ^= state << 13;
		state ^= state >> 7;
		state ^= state << 17;
		let offset = (state as usize % (FILE_SIZE - buf.len())) & !511;
		offsets.push(offset);
	}

	let start = Instant::now();
	for offset in &offsets {
		file.seek(SeekFrom::Start(*offset as u64))?;
		file.read_exact(&mut buf)?;
	}
	let elapsed = start.elapsed();

	log_benchmark_data_with_group(
		"random read, 512 byte reads",
		"us/op",
		micros_per_op(OPS, elapsed),
		"file system read",
	);

	Ok(())
}

/// Stats an existing file repeatedly.
///
/// The FAT layer addresses files by directory entry, so every call re-walks the
/// path. This is what that costs.
fn bench_metadata(name: &str) -> std::io::Result<()> {
	let path = path(name);

	let start = Instant::now();
	for _ in 0..OPS {
		let metadata = fs::metadata(&path)?;
		assert_eq!(metadata.len() as usize, FILE_SIZE);
	}
	let elapsed = start.elapsed();

	log_benchmark_data_with_group(
		"metadata",
		"us/op",
		micros_per_op(OPS, elapsed),
		"file system metadata",
	);

	Ok(())
}

/// Creates and removes small files.
fn bench_create_remove() -> std::io::Result<()> {
	const FILES: usize = 20;

	let start = Instant::now();
	for i in 0..FILES {
		let path = path(&format!("bench{i}.tmp"));
		let mut file = File::create(&path)?;
		file.write_all(b"x")?;
		file.flush()?;
	}
	let create = start.elapsed();

	let start = Instant::now();
	for i in 0..FILES {
		fs::remove_file(path(&format!("bench{i}.tmp")))?;
	}
	let remove = start.elapsed();

	log_benchmark_data_with_group(
		"create file",
		"us/op",
		micros_per_op(FILES, create),
		"file system metadata",
	);
	log_benchmark_data_with_group(
		"remove file",
		"us/op",
		micros_per_op(FILES, remove),
		"file system metadata",
	);

	Ok(())
}

/// Lists the mounted directory.
fn bench_read_dir() -> std::io::Result<()> {
	// The VFS only descends into a mount point when the path carries a
	// trailing separator; without it the root directory itself is listed.
	let dir = format!("{MOUNT}/");

	let start = Instant::now();
	let mut entries = 0;
	for _ in 0..OPS {
		entries = 0;
		for entry in fs::read_dir(&dir)? {
			let _ = entry?;
			entries += 1;
		}
	}
	let elapsed = start.elapsed();

	println!("directory holds {entries} entries");
	log_benchmark_data_with_group(
		"read_dir",
		"us/op",
		micros_per_op(OPS, elapsed),
		"file system metadata",
	);

	Ok(())
}

fn threads() -> usize {
	thread::available_parallelism().map_or(THREADS, |n| THREADS.min(n.get()))
}

/// Reads one shared file from several threads at once.
///
/// Nothing is allocated here, so this isolates how well concurrent readers get
/// through the cache and the driver from any contention over free space.
fn bench_concurrent_read(name: &str, threads: usize) -> std::io::Result<()> {
	let start = Instant::now();

	let read: usize = thread::scope(|scope| {
		let handles: Vec<_> = (0..threads)
			.map(|_| {
				let path = path(name);
				scope.spawn(move || -> std::io::Result<usize> {
					let mut file = File::open(&path)?;
					let mut buf = vec![0u8; THREAD_BUFFER];
					let mut total = 0;
					loop {
						let n = file.read(&mut buf)?;
						if n == 0 {
							return Ok(total);
						}
						total += n;
					}
				})
			})
			.collect();

		handles
			.into_iter()
			.map(|handle| handle.join().expect("reader thread panicked"))
			.sum::<std::io::Result<usize>>()
	})?;

	log_benchmark_data_with_group(
		&format!("concurrent read, {threads} threads"),
		"MiB/s",
		throughput(read, start.elapsed()),
		"file system concurrency",
	);

	Ok(())
}

/// Writes, reads back and removes one file per thread.
///
/// Each thread owns its file, so the file data never overlaps. What the
/// threads do share is the free space accounting and the directory they all
/// create in, which is where concurrent allocation shows up if it is not safe.
fn bench_concurrent_write(threads: usize) -> std::io::Result<()> {
	let start = Instant::now();

	let written: usize = thread::scope(|scope| {
		let handles: Vec<_> = (0..threads)
			.map(|id| {
				scope.spawn(move || -> std::io::Result<usize> {
					let path = path(&format!("mt{id}.bin"));
					let _ = fs::remove_file(&path);

					let chunk: Vec<u8> = (0..THREAD_BUFFER).map(|i| pattern(i + id)).collect();
					let mut file = File::create(&path)?;
					let mut written = 0;
					while written < THREAD_FILE_SIZE {
						let n = usize::min(chunk.len(), THREAD_FILE_SIZE - written);
						file.write_all(&chunk[..n])?;
						written += n;
					}
					file.flush()?;
					drop(file);

					// Reading back is part of the measurement on purpose: a
					// race in the allocator shows up as a file that is short
					// or holds another thread's bytes, and only a comparison
					// catches that.
					let mut file = File::open(&path)?;
					let mut buf = vec![0u8; THREAD_BUFFER];
					let mut read = 0;
					loop {
						let n = file.read(&mut buf)?;
						if n == 0 {
							break;
						}
						for (offset, &byte) in buf[..n].iter().enumerate() {
							if byte != pattern((read + offset) % THREAD_BUFFER + id) {
								return Err(std::io::Error::other(format!(
									"thread {id} read back wrong data at offset {}",
									read + offset
								)));
							}
						}
						read += n;
					}
					if read != written {
						return Err(std::io::Error::other(format!(
							"thread {id} wrote {written} bytes but read back {read}"
						)));
					}

					drop(file);
					fs::remove_file(&path)?;

					Ok(written)
				})
			})
			.collect();

		handles
			.into_iter()
			.map(|handle| handle.join().expect("writer thread panicked"))
			.sum::<std::io::Result<usize>>()
	})?;

	log_benchmark_data_with_group(
		&format!("concurrent write, {threads} threads"),
		"MiB/s",
		throughput(written, start.elapsed()),
		"file system concurrency",
	);

	Ok(())
}

fn run() -> std::io::Result<()> {
	prepare("bench.bin", FILE_SIZE)?;

	for buffer in READ_BUFFERS {
		bench_sequential_read("bench.bin", buffer)?;
	}
	bench_random_read("bench.bin")?;
	bench_metadata("bench.bin")?;

	for buffer in WRITE_BUFFERS {
		bench_sequential_write("bench_write.bin", buffer)?;
	}

	bench_create_remove()?;
	bench_read_dir()?;

	let threads = threads();
	println!("running the concurrent benchmarks on {threads} thread(s)");
	bench_concurrent_read("bench.bin", threads)?;
	bench_concurrent_write(threads)?;

	fs::remove_file(path("bench.bin"))?;

	Ok(())
}

fn main() {
	if !Path::new(MOUNT).exists() {
		println!("{MOUNT} is not mounted -- attach a FAT image to the block device");
		return;
	}

	if let Err(err) = run() {
		println!("benchmark failed: {err}");
	}
}
