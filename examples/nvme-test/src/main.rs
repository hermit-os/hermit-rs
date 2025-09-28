#[cfg(target_os = "hermit")]
use hermit as _;

mod syscalls;
mod tests;
use syscalls::*;
use tests::run_tests;

fn main() {
	println!("Hello, NVMe!");
    // CAREFUL: this example writes to the NVMe drive
	// match example() {
	// 	Err(error) => eprintln!("{error:?}"),
	// 	Ok(()) => println!("Success!"),
	// }
	run_tests();
}

#[allow(dead_code)]
fn example() -> Result<(), SysNvmeError> {
	let namespace_ids = namespace_ids()?;
	println!("Namespace IDs: {namespace_ids:?}.");

	let namespace_id = namespace_ids[0];
	let namespace = namespace(&namespace_id)?;
	println!("Namespace: {namespace:?}");
	println!(
		"Total namespace size: {}",
		namespace.blocks * namespace.block_size
	);

	let maximum_transfer_size = maximum_transfer_size()?;
	println!("Maximum transfer size: {maximum_transfer_size}.");

	let maximum_number_of_io_queue_pairs = maximum_number_of_io_queue_pairs()?;
	println!("Maximum number of I/O queue pairs: {maximum_number_of_io_queue_pairs}.");

	let maximum_queue_entries_supported = maximum_queue_entries_supported()?;
	println!("Maximum queue entries supported: {maximum_queue_entries_supported}.");

	let io_queue_pair_id = create_io_queue_pair(&namespace_id, maximum_queue_entries_supported)?;
	println!(
		"Created IO queue pair with ID {} and {} queue entries for namespace {}.",
		io_queue_pair_id.0, maximum_queue_entries_supported, namespace_id.0
	);

    let length = 16;
    let mut buffer_1 = allocate_buffer(&io_queue_pair_id, length)?;
    for i in 0..length {
        buffer_1[i] = i as u8;
    }

	let logical_block_address = 0;
	write_to_io_queue_pair(&io_queue_pair_id, &buffer_1, logical_block_address)?;
	println!("Wrote to IO queue pair with ID {io_queue_pair_id:?}.");

    let mut buffer_2 = allocate_buffer(&io_queue_pair_id, length)?;
	read_from_io_queue_pair(&io_queue_pair_id, &mut buffer_2, logical_block_address)?;
	println!("Read from IO queue pair with ID {io_queue_pair_id:?}.");

	println!("buffer_1: {:?}", &buffer_1[0..length]);
	println!("buffer_2: {:?}", &buffer_2[0..length]);

    deallocate_buffer(&io_queue_pair_id, buffer_1)?;
    deallocate_buffer(&io_queue_pair_id, buffer_2)?;
	println!("Deallocated buffers.");

	delete_io_queue_pair(io_queue_pair_id)?;
	println!("Deleted IO queue pair.");
	Ok(())
}
