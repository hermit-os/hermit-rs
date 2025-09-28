use std::thread;
use std::time::{Duration, Instant};

use vroom::NamespaceId;

use crate::syscalls::*;

pub fn run_tests() {
	println!("Start running tests.");
	test_namespaces();
	// test_maximum_transfer_size();
	// test_maximum_number_of_io_queue_pairs();
	// test_maximum_queue_entries_supported();
	// test_io_queue_pair_creation();
	// test_submit_write_read().unwrap();
	// test_full_namespace_write_read().unwrap();
	// clear_namespaces();
	// fill_namespaces();
	// benchmark_seqr_1_thread(Duration::from_secs(60), 32);
	// benchmark_randr_1_thread(Duration::from_secs(120), 32);
	// benchmark_randw_1_thread(Duration::from_secs(900), 32);
	// benchmark_randw_n_threads(Duration::from_secs(5), 2, 32); // Does not work properly
	println!("Tests ran successfully.");
}

#[allow(dead_code)]
fn test_namespaces() {
	let result = namespace_ids();
	assert!(
		result.is_ok(),
		"Could not get namespace IDs. Please verify that an NVMe device is available."
	);

	let namespace_ids = result.unwrap();
	namespace_ids.iter().for_each(|namespace_id| {
		let namespace = namespace(&namespace_id);
		dbg!(&namespace);
		assert!(namespace.is_ok());
	});

	let invalid_namespace_id =
		NamespaceId(namespace_ids.iter().max().map(|id| id.0 + 1).unwrap_or(0));
	let invalid = namespace(&invalid_namespace_id);
	assert!(invalid.is_err());
}

#[allow(dead_code)]
fn test_maximum_transfer_size() {
	let result = maximum_transfer_size();
	dbg!(&result);
	assert!(
		result.is_ok(),
		"Could not get the maximum transfer size. Please verify that an NVMe device is available."
	);
}

#[allow(dead_code)]
fn test_maximum_number_of_io_queue_pairs() {
	let result = maximum_number_of_io_queue_pairs();
	dbg!(&result);
	assert!(
		result.is_ok(),
		"Could not get the maximum number of I/O queue pairs. Please verify that an NVMe device is available."
	);
}

#[allow(dead_code)]
fn test_maximum_queue_entries_supported() {
	let result = maximum_queue_entries_supported();
	assert!(
		result.is_ok(),
		"Could not get the maximum number of supported queue entries. Please verify that an NVMe device is available."
	);
	let maximum_queue_entries_supported = result.unwrap();
	dbg!(&maximum_queue_entries_supported);
	assert!(maximum_queue_entries_supported >= 2);
}

#[allow(dead_code)]
fn test_io_queue_pair_creation() {
	let namespace_ids: Vec<NamespaceId> = namespace_ids().unwrap();
	let max_entries: u32 = maximum_queue_entries_supported().unwrap();
	for namespace_id in namespace_ids {
		let result = create_io_queue_pair(&namespace_id, 0);
		assert!(result.is_err());
		let result = create_io_queue_pair(&namespace_id, 1);
		assert!(result.is_err());

		let result = create_io_queue_pair(&namespace_id, 2);
		assert!(result.is_ok());
		let result = delete_io_queue_pair(result.unwrap());
		assert!(result.is_ok());

		let result = create_io_queue_pair(&namespace_id, (max_entries / 2).min(2));
		assert!(result.is_ok());
		let result = delete_io_queue_pair(result.unwrap());
		assert!(result.is_ok());

		let result = create_io_queue_pair(&namespace_id, max_entries);
		assert!(result.is_ok());
		let result = delete_io_queue_pair(result.unwrap());
		assert!(result.is_ok());

		if max_entries < u32::MAX {
			let result = create_io_queue_pair(&namespace_id, max_entries + 1);
			assert!(result.is_err());
			let result = create_io_queue_pair(&namespace_id, u32::MAX);
			assert!(result.is_err());
		}

		let max_number_of_queue_pairs = maximum_number_of_io_queue_pairs().unwrap();
		let mut queue_pairs = Vec::new();

		(0..max_number_of_queue_pairs).for_each(|_| {
			let result = create_io_queue_pair(&namespace_id, max_entries);
			assert!(result.is_ok());
			queue_pairs.push(result.unwrap())
		});

		let result = create_io_queue_pair(&namespace_id, max_entries);
		assert!(result.is_err());

		queue_pairs.into_iter().for_each(|queue_pair| {
			let result = delete_io_queue_pair(queue_pair);
			assert!(result.is_ok());
		});
	}
}

#[allow(dead_code)]
fn test_submit_write_read() -> Result<(), SysNvmeError> {
	let namespace_ids: Vec<NamespaceId> = namespace_ids()?;
	let namespace_id: &NamespaceId = namespace_ids.first().unwrap();
	let namespace = namespace(namespace_id)?;
	let buffer_length: usize = 4096;
	let max_entries = maximum_queue_entries_supported()?;
	let io_queue_pair_id = create_io_queue_pair(namespace_id, max_entries)?;

	let mut source_buffer = allocate_buffer(&io_queue_pair_id, buffer_length)?;
	(0..buffer_length).for_each(|i| source_buffer[i] = rand::random::<u8>());

	let mut dest_buffer = allocate_buffer(&io_queue_pair_id, buffer_length)?;
	(0..buffer_length).for_each(|i| dest_buffer[i] = rand::random::<u8>());

	let alignment = buffer_length as u64 / namespace.block_size;
	let max_logical_block_address = namespace.blocks - alignment;
	let logical_block_address =
		rand::random_range(0..(max_logical_block_address / alignment)) * alignment;

	submit_write_to_io_queue_pair(&io_queue_pair_id, &source_buffer, logical_block_address)?;
	complete_io_with_io_queue_pair(&io_queue_pair_id)?;

	submit_read_to_io_queue_pair(&io_queue_pair_id, &mut dest_buffer, logical_block_address)?;
	complete_io_with_io_queue_pair(&io_queue_pair_id)?;

	for i in 0..buffer_length {
		assert!(source_buffer[i] == dest_buffer[i]);
	}

	deallocate_buffer(&io_queue_pair_id, source_buffer)?;
	deallocate_buffer(&io_queue_pair_id, dest_buffer)?;
	delete_io_queue_pair(io_queue_pair_id)?;
	Ok(())
}

#[allow(dead_code)]
fn test_full_namespace_write_read() -> Result<(), SysNvmeError> {
	let namespace_ids: Vec<NamespaceId> = namespace_ids()?;
	let namespace_id: &NamespaceId = namespace_ids.first().unwrap();
	let namespace = namespace(namespace_id)?;
	let max_entries: u32 = maximum_queue_entries_supported()?;
	let io_queue_pair_id = create_io_queue_pair(namespace_id, max_entries)?;

	let page_size = 4096;
	let block_size = namespace.block_size;
	let blocks = namespace.blocks;
	let namespace_size = blocks * block_size;

	let buffer_length = 8192;
	let required_operations = namespace_size / buffer_length as u64;
	let remainder = (namespace_size % buffer_length as u64) as usize;
	let blocks_per_operation = buffer_length as u64 / block_size;
	let remainder_operations = remainder / page_size;

	let mut source_buffer = allocate_buffer(&io_queue_pair_id, buffer_length)?;
	(0..buffer_length).for_each(|i| source_buffer[i] = rand::random::<u8>());

	let mut destination_buffer = allocate_buffer(&io_queue_pair_id, buffer_length)?;
	(0..buffer_length).for_each(|i| destination_buffer[i] = rand::random::<u8>());

	let mut random_buffer = allocate_buffer(&io_queue_pair_id, buffer_length)?;
	(0..buffer_length).for_each(|i| random_buffer[i] = rand::random::<u8>());

	println!(
        "Testing full write and read of name space {} with {blocks} blocks and a block size of {block_size}.",
        namespace_id.0
    );
	println!(
		"{required_operations} operations are required with a buffer size of {buffer_length}."
	);
	println!("{remainder_operations} operations are required for the remainder with a buffer size of {page_size}.");
	let start_time = Instant::now();
	for i in 0..required_operations {
		for j in 0..buffer_length {
			destination_buffer[j] = random_buffer[j];
		}
		let logical_block_address = i * blocks_per_operation;
		write_to_io_queue_pair(&io_queue_pair_id, &source_buffer, logical_block_address)?;
		read_from_io_queue_pair(
			&io_queue_pair_id,
			&mut destination_buffer,
			logical_block_address,
		)?;
		for j in 0..buffer_length {
			assert!(source_buffer[j] == destination_buffer[j]);
		}
	}

	deallocate_buffer(&io_queue_pair_id, source_buffer)?;
	deallocate_buffer(&io_queue_pair_id, destination_buffer)?;

	if remainder != 0 {
		let mut source_buffer = allocate_buffer(&io_queue_pair_id, page_size)?;
		(0..page_size).for_each(|i| source_buffer[i] = rand::random::<u8>());

		let mut destination_buffer = allocate_buffer(&io_queue_pair_id, page_size)?;
		(0..page_size).for_each(|i| destination_buffer[i] = rand::random::<u8>());

		for i in 0..page_size {
			destination_buffer[i] = random_buffer[i];
		}
		for i in 0..remainder_operations as u64 {
			let logical_block_address =
				required_operations * blocks_per_operation + i * page_size as u64;
			dbg!(logical_block_address);
			write_to_io_queue_pair(&io_queue_pair_id, &source_buffer, logical_block_address)?;
			read_from_io_queue_pair(
				&io_queue_pair_id,
				&mut destination_buffer,
				logical_block_address,
			)?;
			for j in 0..page_size {
				assert!(source_buffer[j] == destination_buffer[j]);
			}
		}
		deallocate_buffer(&io_queue_pair_id, source_buffer)?;
		deallocate_buffer(&io_queue_pair_id, destination_buffer)?;
	}
	let elapsed_time = start_time.elapsed();
	println!(
		"Finished testing name space {} in {:.2} seconds",
		namespace_id.0,
		elapsed_time.as_secs_f64()
	);

	deallocate_buffer(&io_queue_pair_id, random_buffer)?;

	Ok(())
}

#[allow(dead_code)]
fn clear_namespaces() {
	for namespace_id in namespace_ids().unwrap() {
		clear_namespace(&namespace_id).unwrap();
	}
}

#[allow(dead_code)]
fn fill_namespaces() {
	let namespace_ids: Vec<NamespaceId> = namespace_ids().unwrap();
	let buffer_length: usize = 4096;
	let max_entries = maximum_queue_entries_supported().unwrap();

	for namespace_id in namespace_ids {
		let namespace = namespace(&namespace_id).unwrap();
		let io_queue_pair_id = create_io_queue_pair(&namespace_id, max_entries).unwrap();
		let mut buffer = allocate_buffer(&io_queue_pair_id, buffer_length).unwrap();
		(0..buffer_length).for_each(|i| buffer[i] = rand::random::<u8>());

		let alignment = buffer_length as u64 / namespace.block_size;
		let max_logical_block_address = namespace.blocks - alignment;

		println!(
			"Started filling name space {} with {} blocks",
			namespace_id.0, namespace.blocks
		);
		let start_time = Instant::now();
		for i in 0..(max_logical_block_address / alignment) {
			let logical_block_address = i * alignment;
			write_to_io_queue_pair(&io_queue_pair_id, &buffer, logical_block_address).unwrap();
		}
		let elapsed_time = start_time.elapsed();

		println!(
			"Finished filling name space {} in {:.2} seconds",
			namespace_id.0,
			elapsed_time.as_secs_f64()
		);

		deallocate_buffer(&io_queue_pair_id, buffer).unwrap();
		delete_io_queue_pair(io_queue_pair_id).unwrap();
	}
}

#[allow(dead_code)]
fn benchmark_seqr_1_thread(duration: Duration, queue_depth: u32) {
	let namespace_ids: Vec<NamespaceId> = namespace_ids().unwrap();
	let namespace_id: &NamespaceId = namespace_ids.first().unwrap();
	let namespace = namespace(namespace_id).unwrap();
	let buffer_length: usize = 4096;
	let max_entries = maximum_queue_entries_supported().unwrap();
	assert!(queue_depth <= max_entries);
	let io_queue_pair_id = create_io_queue_pair(namespace_id, max_entries).unwrap();

	let mut buffer = allocate_buffer(&io_queue_pair_id, buffer_length).unwrap();
	(0..buffer_length).for_each(|i| buffer[i] = rand::random::<u8>());

	let mut active_submissions: u32 = 0;
	let mut io_operations = 0;
	// let mut last_measurement_time = Duration::ZERO;
	// let mut vec_operations = Vec::new();
	let alignment = buffer_length as u64 / namespace.block_size;
	let max_logical_block_address = namespace.blocks - alignment;

	println!(
		"Benchmark sequential read for {} s; queue depth {queue_depth}",
		duration.as_secs()
	);
	let start_time = Instant::now();
	for i in 0..(max_logical_block_address / alignment) {
		if start_time.elapsed() >= duration {
			break;
		}
		let logical_block_address = i * alignment;
		while let Ok(_) = complete_io_with_io_queue_pair(&io_queue_pair_id) {
			active_submissions -= 1;
			io_operations += 1;
		}
		if active_submissions < queue_depth {
			submit_read_to_io_queue_pair(&io_queue_pair_id, &mut buffer, logical_block_address)
				.unwrap();
			active_submissions += 1;
		}
		// let elapsed_time = start_time.elapsed();
		// if elapsed_time.as_secs() > last_measurement_time.as_secs() {
		// 	vec_operations.push((elapsed_time.as_secs_f64(), io_operations));
		// 	last_measurement_time = elapsed_time;
		// }
	}
	while active_submissions > 0 {
		if let Ok(_) = complete_io_with_io_queue_pair(&io_queue_pair_id) {
			active_submissions -= 1;
			io_operations += 1;
		}
	}
	let elapsed_time = start_time.elapsed();
	let iops = io_operations as f64 / elapsed_time.as_secs_f64();
	let mbps = iops * buffer_length as f64 / 1_000_000_f64;
	println!("Elapsed time: {:.2} s", elapsed_time.as_secs_f64());
	println!("Read operations: {io_operations}");
	println!("IOPS: {iops:.2}");
	println!("MB/s: {mbps:.2}");

	// println!("(seconds,kIOPS)");
	// let absolute_times: Vec<f64> = vec_operations
	// 	.iter()
	// 	.skip(1)
	// 	.map(|(time, _)| *time)
	// 	.collect();
	// let delta_times: Vec<f64> = vec_operations
	// 	.windows(2)
	// 	.map(|slice| slice[1].0 - slice[0].0)
	// 	.collect();
	// let delta_operations: Vec<u64> = vec_operations
	// 	.windows(2)
	// 	.map(|slice| slice[1].1 - slice[0].1)
	// 	.collect();
	// let average_points = 4;
	// for i in 0..absolute_times.len() {
	// 	let previous_points = (average_points - 1).min(i);
	// 	let mut total_delta_time = 0f64;
	// 	let mut total_delta_operations = 0u64;
	// 	for j in 0..previous_points + 1 {
	// 		total_delta_time += delta_times[i - j];
	// 		total_delta_operations += delta_operations[i - j];
	// 	}
	// 	let average_delta_time = total_delta_time / (previous_points + 1) as f64;
	// 	let average_delta_operations = total_delta_operations as f64 / (previous_points + 1) as f64;

	// 	let elapsed_time = absolute_times[i];
	// 	let average_kiops = (average_delta_operations / average_delta_time) / 1000f64;
	// 	println!("({elapsed_time:.0},{average_kiops:.3})");
	// }

	deallocate_buffer(&io_queue_pair_id, buffer).unwrap();
	delete_io_queue_pair(io_queue_pair_id).unwrap();
}

#[allow(dead_code)]
fn benchmark_randr_1_thread(duration: Duration, queue_depth: u32) {
	let namespace_ids: Vec<NamespaceId> = namespace_ids().unwrap();
	let namespace_id: &NamespaceId = namespace_ids.first().unwrap();
	let namespace = namespace(namespace_id).unwrap();
	let buffer_length: usize = 4096;
	let max_entries = maximum_queue_entries_supported().unwrap();
	assert!(queue_depth <= max_entries);
	let io_queue_pair_id = create_io_queue_pair(namespace_id, max_entries).unwrap();

	let mut buffer = allocate_buffer(&io_queue_pair_id, buffer_length).unwrap();
	(0..buffer_length).for_each(|i| buffer[i] = rand::random::<u8>());

	let mut active_submissions: u32 = 0;
	let mut io_operations = 0;
	let mut last_measurement_time = Duration::ZERO;
	let mut vec_operations = Vec::new();
	let alignment = buffer_length as u64 / namespace.block_size;
	let max_logical_block_address = namespace.blocks - alignment;

	println!(
		"Benchmark random read for {} s; queue depth {queue_depth}",
		duration.as_secs()
	);
	let start_time = Instant::now();
	while start_time.elapsed() < duration {
		let logical_block_address =
			rand::random_range(0..(max_logical_block_address / alignment)) * alignment;
		while let Ok(_) = complete_io_with_io_queue_pair(&io_queue_pair_id) {
			active_submissions -= 1;
			io_operations += 1;
		}
		if active_submissions < queue_depth {
			submit_read_to_io_queue_pair(&io_queue_pair_id, &mut buffer, logical_block_address)
				.unwrap();
			active_submissions += 1;
		}
		let elapsed_time = start_time.elapsed();
		if elapsed_time.as_secs() > last_measurement_time.as_secs() {
			vec_operations.push((elapsed_time.as_secs_f64(), io_operations));
			last_measurement_time = elapsed_time;
		}
	}
	while active_submissions > 0 {
		if let Ok(_) = complete_io_with_io_queue_pair(&io_queue_pair_id) {
			active_submissions -= 1;
			io_operations += 1;
		}
	}
	let elapsed_time = start_time.elapsed();
	let iops = io_operations as f64 / elapsed_time.as_secs_f64();
	println!("Elapsed time: {:.2} s", elapsed_time.as_secs_f64());
	println!("Read operations: {io_operations}");
	println!("IOPS: {iops:.2}");

	println!("(seconds,kIOPS)");
	let absolute_times: Vec<f64> = vec_operations
		.iter()
		.skip(1)
		.map(|(time, _)| *time)
		.collect();
	let delta_times: Vec<f64> = vec_operations
		.windows(2)
		.map(|slice| slice[1].0 - slice[0].0)
		.collect();
	let delta_operations: Vec<u64> = vec_operations
		.windows(2)
		.map(|slice| slice[1].1 - slice[0].1)
		.collect();
	let average_points = 4;
	for i in 0..absolute_times.len() {
		let previous_points = (average_points - 1).min(i);
		let mut total_delta_time = 0f64;
		let mut total_delta_operations = 0u64;
		for j in 0..previous_points + 1 {
			total_delta_time += delta_times[i - j];
			total_delta_operations += delta_operations[i - j];
		}
		let average_delta_time = total_delta_time / (previous_points + 1) as f64;
		let average_delta_operations = total_delta_operations as f64 / (previous_points + 1) as f64;

		let elapsed_time = absolute_times[i];
		let average_kiops = (average_delta_operations / average_delta_time) / 1000f64;
		println!("({elapsed_time:.0},{average_kiops:.3})");
	}

	deallocate_buffer(&io_queue_pair_id, buffer).unwrap();
	delete_io_queue_pair(io_queue_pair_id).unwrap();
}

/// CAUTION: this will overwrite the NVMe device with random data!
#[allow(dead_code)]
fn benchmark_randw_1_thread(duration: Duration, queue_depth: u32) {
	let namespace_ids: Vec<NamespaceId> = namespace_ids().unwrap();
	let namespace_id: &NamespaceId = namespace_ids.first().unwrap();
	let namespace = namespace(namespace_id).unwrap();
	let buffer_length: usize = 4096;
	let max_entries = maximum_queue_entries_supported().unwrap();
	assert!(queue_depth <= max_entries);
	let io_queue_pair_id = create_io_queue_pair(namespace_id, max_entries).unwrap();

	let mut buffer = allocate_buffer(&io_queue_pair_id, buffer_length).unwrap();
	(0..buffer_length).for_each(|i| buffer[i] = rand::random::<u8>());

	let mut active_submissions: u32 = 0;
	let mut io_operations: u64 = 0;
	let mut last_measurement_time = Duration::ZERO;
	let mut vec_operations = Vec::new();
	let alignment = buffer_length as u64 / namespace.block_size;
	let max_logical_block_address = namespace.blocks - alignment;

	clear_namespace(namespace_id).unwrap();

	println!(
		"Benchmark random write for {} s; queue depth {queue_depth}",
		duration.as_secs()
	);
	let start_time = Instant::now();
	while start_time.elapsed() < duration {
		let logical_block_address =
			rand::random_range(0..(max_logical_block_address / alignment)) * alignment;
		while let Ok(_) = complete_io_with_io_queue_pair(&io_queue_pair_id) {
			active_submissions -= 1;
			io_operations += 1;
		}
		if active_submissions < queue_depth {
			submit_write_to_io_queue_pair(&io_queue_pair_id, &mut buffer, logical_block_address)
				.unwrap();
			active_submissions += 1;
		}
		let elapsed_time = start_time.elapsed();
		if elapsed_time.as_secs() > last_measurement_time.as_secs() {
			vec_operations.push((elapsed_time.as_secs_f64(), io_operations));
			last_measurement_time = elapsed_time;
		}
	}
	while active_submissions > 0 {
		if let Ok(_) = complete_io_with_io_queue_pair(&io_queue_pair_id) {
			active_submissions -= 1;
			io_operations += 1;
		}
	}
	let elapsed_time = start_time.elapsed();
	let iops = io_operations as f64 / elapsed_time.as_secs_f64();
	println!("Elapsed time: {:.2} s", elapsed_time.as_secs_f64());
	println!("Write operations: {io_operations}");
	println!("IOPS: {iops:.2}");

	println!("(seconds,kIOPS)");
	let absolute_times: Vec<f64> = vec_operations
		.iter()
		.skip(1)
		.map(|(time, _)| *time)
		.collect();
	let delta_times: Vec<f64> = vec_operations
		.windows(2)
		.map(|slice| slice[1].0 - slice[0].0)
		.collect();
	let delta_operations: Vec<u64> = vec_operations
		.windows(2)
		.map(|slice| slice[1].1 - slice[0].1)
		.collect();
	let average_points = 6;
	for i in 0..absolute_times.len() {
		let previous_points = (average_points - 1).min(i);
		let mut total_delta_time = 0f64;
		let mut total_delta_operations = 0u64;
		for j in 0..previous_points + 1 {
			total_delta_time += delta_times[i - j];
			total_delta_operations += delta_operations[i - j];
		}
		let average_delta_time = total_delta_time / (previous_points + 1) as f64;
		let average_delta_operations = total_delta_operations as f64 / (previous_points + 1) as f64;

		let elapsed_time = absolute_times[i];
		let average_kiops = (average_delta_operations / average_delta_time) / 1000f64;
		println!("({elapsed_time:.0},{average_kiops:.3})");
	}

	deallocate_buffer(&io_queue_pair_id, buffer).unwrap();
	delete_io_queue_pair(io_queue_pair_id).unwrap();
}

/// CAUTION: this will overwrite the NVMe device with random data!
/// Hermit uses a cooperative scheduler that does not play nicely with
/// parallelizing these workloads to many threads.
/// The threads need to manually yield which seems to cause big overhead.
#[allow(dead_code)]
fn benchmark_randw_n_threads(duration: Duration, number_of_threads: u8, queue_depth: u32) {
	println!(
		"Benchmark random write for {} s; threads: {number_of_threads}, queue depth {queue_depth}",
		duration.as_secs()
	);
	let mut io_queue_pair_ids = Vec::new();
	let mut threads = Vec::new();

	let namespace_ids: Vec<NamespaceId> = namespace_ids().unwrap();
	let namespace_id: &NamespaceId = namespace_ids.first().unwrap();
	let namespace = namespace(namespace_id).unwrap();
	let buffer_length: usize = 4096;
	let alignment = buffer_length as u64 / namespace.block_size;
	let max_logical_block_address = namespace.blocks - alignment;
	let max_entries = maximum_queue_entries_supported().unwrap();
	assert!(queue_depth <= max_entries);

	clear_namespace(namespace_id).unwrap();

	for _ in 0..number_of_threads {
		let io_queue_pair_id = create_io_queue_pair(&namespace_id, max_entries).unwrap();
		io_queue_pair_ids.push(io_queue_pair_id);
	}
	println!("{io_queue_pair_ids:?}");
	for i in 0..number_of_threads {
		let io_queue_pair_id = io_queue_pair_ids[i as usize];
		let mut buffer = allocate_buffer(&io_queue_pair_id, buffer_length).unwrap();
		(0..buffer_length).for_each(|i| buffer[i] = rand::random::<u8>());

		// returns operations and IOPS
		let handle = thread::spawn(move || -> (u64, f64) {
			println!("T{i}: spawned");
			let mut active_submissions: u32 = 0;
			let mut io_operations = 0;
			let mut yield_counter: u64 = 0;
			let mut last_yield_time = Duration::ZERO;
			let yield_time_delta = Duration::from_millis(1000);

			thread::yield_now();

			let start_time = Instant::now();
			while start_time.elapsed() < duration {
				let logical_block_address =
					rand::random_range(0..(max_logical_block_address / alignment)) * alignment;
				while let Ok(_) = complete_io_with_io_queue_pair(&io_queue_pair_id) {
					active_submissions -= 1;
					io_operations += 1;
				}
				if active_submissions < queue_depth {
					submit_write_to_io_queue_pair(
						&io_queue_pair_id,
						&mut buffer,
						logical_block_address,
					)
					.unwrap();
					active_submissions += 1;
				}
				if start_time.elapsed() > last_yield_time + yield_time_delta {
					last_yield_time = start_time.elapsed();
					yield_counter += 1;
					thread::yield_now();
				}
			}
			while active_submissions > 0 {
				if let Ok(_) = complete_io_with_io_queue_pair(&io_queue_pair_id) {
					active_submissions -= 1;
					io_operations += 1;
				}
			}
			let elapsed_time = start_time.elapsed();
			println!("T{i}: yield counter {yield_counter}");
			println!("T{i}: elapsed time {:.2}", elapsed_time.as_secs_f64());

			deallocate_buffer(&io_queue_pair_id, buffer).unwrap();
			println!("T{i}: deallocated buffer");

			let io_operations_per_second = io_operations as f64 / elapsed_time.as_secs_f64();
			(io_operations, io_operations_per_second)
		});
		threads.push(handle);
	}

	let global_start_time = Instant::now();

	let (io_operations, io_operations_per_second) =
		threads.into_iter().fold((0, 0.), |accumulator, thread| {
			let result = thread
				.join()
				.expect("The thread creation or execution failed!");
			(accumulator.0 + result.0, accumulator.1 + result.1)
		});

	let globally_elapsed_time = global_start_time.elapsed();
	println!(
		"Globally elapsed seconds: {:.2}",
		globally_elapsed_time.as_secs_f64()
	);

	for i in 0..number_of_threads {
		delete_io_queue_pair(io_queue_pair_ids[i as usize]).unwrap();
		println!("T{i}: deleted IOQPID {:?}", io_queue_pair_ids[i as usize]);
	}

	println!("Write operations: {io_operations}");
	println!("IOPS: {io_operations_per_second:.2}");
}
