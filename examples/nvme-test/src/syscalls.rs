use vroom::{Dma, IoQueuePairId, Namespace, NamespaceId};

pub fn namespace_ids() -> Result<Vec<NamespaceId>, SysNvmeError> {
	let mut number_of_namespaces: u32 = 0;
	let exit_code = unsafe { sys_nvme_number_of_namespaces(&mut number_of_namespaces) };
	if exit_code != 0 {
		return Err(SysNvmeError::from(exit_code));
	}
	let mut namespace_ids: Vec<NamespaceId> = Vec::with_capacity(number_of_namespaces as usize);
	let exit_code =
		unsafe { sys_nvme_namespace_ids(namespace_ids.as_mut_ptr(), number_of_namespaces) };

	let namespace_ids_pointer = namespace_ids.as_mut_ptr();
	core::mem::forget(namespace_ids); // prevents the Vec from being dropped and deallocated
	let namespace_ids = unsafe {
		Vec::from_raw_parts(
			namespace_ids_pointer,
			number_of_namespaces as usize,
			number_of_namespaces as usize,
		)
	};
	match exit_code {
		0 => Ok(namespace_ids),
		n => Err(SysNvmeError::from(n)),
	}
}

pub fn namespace(namespace_id: &NamespaceId) -> Result<Namespace, SysNvmeError> {
	let mut namespace: Namespace = Namespace {
		id: NamespaceId(0),
		blocks: 0,
		block_size: 0,
	};
	let exit_code = unsafe { sys_nvme_namespace(namespace_id, &mut namespace) };
	match exit_code {
		0 => Ok(namespace),
		n => Err(SysNvmeError::from(n)),
	}
}

pub fn clear_namespace(namespace_id: &NamespaceId) -> Result<(), SysNvmeError> {
	let exit_code = unsafe { sys_nvme_clear_namespace(namespace_id) };
	match exit_code {
		0 => Ok(()),
		n => Err(SysNvmeError::from(n)),
	}
}

pub fn maximum_transfer_size() -> Result<usize, SysNvmeError> {
	let mut maximum_transfer_size: usize = 0;
	let exit_code = unsafe { sys_nvme_maximum_transfer_size(&mut maximum_transfer_size) };
	match exit_code {
		0 => Ok(maximum_transfer_size),
		n => Err(SysNvmeError::from(n)),
	}
}

pub fn maximum_number_of_io_queue_pairs() -> Result<u16, SysNvmeError> {
	let mut maximum_number_of_io_queue_pairs: u16 = 0;
	let exit_code =
		unsafe { sys_nvme_maximum_number_of_io_queue_pairs(&mut maximum_number_of_io_queue_pairs) };
	match exit_code {
		0 => Ok(maximum_number_of_io_queue_pairs),
		n => Err(SysNvmeError::from(n)),
	}
}

pub fn maximum_queue_entries_supported() -> Result<u32, SysNvmeError> {
	let mut maximum_queue_entries_supported: u32 = 0;
	let exit_code =
		unsafe { sys_nvme_maximum_queue_entries_supported(&mut maximum_queue_entries_supported) };
	match exit_code {
		0 => Ok(maximum_queue_entries_supported),
		n => Err(SysNvmeError::from(n)),
	}
}

pub fn create_io_queue_pair(
	namespace_id: &NamespaceId,
	number_of_entries: u32,
) -> Result<IoQueuePairId, SysNvmeError> {
	let mut result: IoQueuePairId = IoQueuePairId(0);
	let exit_code =
		unsafe { sys_nvme_create_io_queue_pair(namespace_id, number_of_entries, &mut result) };
	match exit_code {
		0 => Ok(result),
		n => Err(SysNvmeError::from(n)),
	}
}

pub fn delete_io_queue_pair(io_queue_pair_id: IoQueuePairId) -> Result<(), SysNvmeError> {
	let exit_code = unsafe { sys_nvme_delete_io_queue_pair(io_queue_pair_id) };
	match exit_code {
		0 => Ok(()),
		n => Err(SysNvmeError::from(n)),
	}
}

pub fn allocate_buffer(
	io_queue_pair_id: &IoQueuePairId,
	number_of_elements: usize,
) -> Result<Dma<u8>, SysNvmeError> {
	let mut result: Dma<u8> = unsafe { Dma::new_uninitialized() };
	let exit_code =
		unsafe { sys_nvme_allocate_buffer(io_queue_pair_id, number_of_elements, &mut result) };
	match exit_code {
		0 => Ok(result),
		n => Err(SysNvmeError::from(n)),
	}
}

pub fn deallocate_buffer(
	io_queue_pair_id: &IoQueuePairId,
	mut buffer: Dma<u8>,
) -> Result<(), SysNvmeError> {
	let exit_code =
		unsafe { sys_nvme_deallocate_buffer(io_queue_pair_id, &mut buffer as *mut Dma<u8>) };
	match exit_code {
		0 => Ok(()),
		n => Err(SysNvmeError::from(n)),
	}
}

pub fn read_from_io_queue_pair(
	io_queue_pair_id: &IoQueuePairId,
	buffer: &mut Dma<u8>,
	logical_block_address: u64,
) -> Result<(), SysNvmeError> {
	let exit_code = unsafe {
		sys_nvme_read_from_io_queue_pair(io_queue_pair_id, buffer, logical_block_address)
	};
	match exit_code {
		0 => Ok(()),
		n => Err(SysNvmeError::from(n)),
	}
}

pub fn write_to_io_queue_pair(
	io_queue_pair_id: &IoQueuePairId,
	buffer: &Dma<u8>,
	logical_block_address: u64,
) -> Result<(), SysNvmeError> {
	let exit_code =
		unsafe { sys_nvme_write_to_io_queue_pair(io_queue_pair_id, buffer, logical_block_address) };
	match exit_code {
		0 => Ok(()),
		n => Err(SysNvmeError::from(n)),
	}
}

pub fn submit_read_to_io_queue_pair(
	io_queue_pair_id: &IoQueuePairId,
	buffer: &mut Dma<u8>,
	logical_block_address: u64,
) -> Result<(), SysNvmeError> {
	let exit_code = unsafe {
		sys_nvme_submit_read_to_io_queue_pair(io_queue_pair_id, buffer, logical_block_address)
	};
	match exit_code {
		0 => Ok(()),
		n => Err(SysNvmeError::from(n)),
	}
}

pub fn submit_write_to_io_queue_pair(
	io_queue_pair_id: &IoQueuePairId,
	buffer: &Dma<u8>,
	logical_block_address: u64,
) -> Result<(), SysNvmeError> {
	let exit_code = unsafe {
		sys_nvme_submit_write_to_io_queue_pair(io_queue_pair_id, buffer, logical_block_address)
	};
	match exit_code {
		0 => Ok(()),
		n => Err(SysNvmeError::from(n)),
	}
}

pub fn complete_io_with_io_queue_pair(
	io_queue_pair_id: &IoQueuePairId,
) -> Result<(), SysNvmeError> {
	let exit_code = unsafe { sys_nvme_complete_io_with_io_queue_pair(io_queue_pair_id) };
	match exit_code {
		0 => Ok(()),
		n => Err(SysNvmeError::from(n)),
	}
}

unsafe extern "C" {
	#[link_name = "sys_nvme_number_of_namespaces"]
	fn sys_nvme_number_of_namespaces(result: *mut u32) -> usize;

	#[link_name = "sys_nvme_namespace_ids"]
	fn sys_nvme_namespace_ids(vec_pointer: *mut NamespaceId, length: u32) -> usize;

	#[link_name = "sys_nvme_namespace"]
	fn sys_nvme_namespace(namespace_id: &NamespaceId, result: *mut Namespace) -> usize;

	#[link_name = "sys_nvme_clear_namespace"]
	fn sys_nvme_clear_namespace(namespace_id: &NamespaceId) -> usize;

	#[link_name = "sys_nvme_maximum_transfer_size"]
	fn sys_nvme_maximum_transfer_size(result: *mut usize) -> usize;

	#[link_name = "sys_nvme_maximum_number_of_io_queue_pairs"]
	fn sys_nvme_maximum_number_of_io_queue_pairs(result: *mut u16) -> usize;

	#[link_name = "sys_nvme_maximum_queue_entries_supported"]
	fn sys_nvme_maximum_queue_entries_supported(result: *mut u32) -> usize;

	#[link_name = "sys_nvme_create_io_queue_pair"]
	fn sys_nvme_create_io_queue_pair(
		namespace_id: &NamespaceId,
		number_of_entries: u32,
		resulting_io_queue_pair_id: *mut IoQueuePairId,
	) -> usize;

	#[link_name = "sys_nvme_delete_io_queue_pair"]
	fn sys_nvme_delete_io_queue_pair(io_queue_pair_id: IoQueuePairId) -> usize;

	#[link_name = "sys_nvme_allocate_buffer"]
	fn sys_nvme_allocate_buffer(
		io_queue_pair_id: &IoQueuePairId,
		size: usize,
		resulting_buffer: *mut Dma<u8>,
	) -> usize;

	#[link_name = "sys_nvme_deallocate_buffer"]
	fn sys_nvme_deallocate_buffer(io_queue_pair_id: &IoQueuePairId, buffer: *mut Dma<u8>) -> usize;

	#[link_name = "sys_nvme_read_from_io_queue_pair"]
	fn sys_nvme_read_from_io_queue_pair(
		io_queue_pair_id: &IoQueuePairId,
		buffer: *mut Dma<u8>,
		logical_block_address: u64,
	) -> usize;

	#[link_name = "sys_nvme_write_to_io_queue_pair"]
	fn sys_nvme_write_to_io_queue_pair(
		io_queue_pair_id: &IoQueuePairId,
		buffer: *const Dma<u8>,
		logical_block_address: u64,
	) -> usize;

	#[link_name = "sys_nvme_submit_read_to_io_queue_pair"]
	fn sys_nvme_submit_read_to_io_queue_pair(
		io_queue_pair_id: &IoQueuePairId,
		buffer: *mut Dma<u8>,
		logical_block_address: u64,
	) -> usize;

	#[link_name = "sys_nvme_submit_write_to_io_queue_pair"]
	fn sys_nvme_submit_write_to_io_queue_pair(
		io_queue_pair_id: &IoQueuePairId,
		buffer: *const Dma<u8>,
		logical_block_address: u64,
	) -> usize;

	#[link_name = "sys_nvme_complete_io_with_io_queue_pair"]
	fn sys_nvme_complete_io_with_io_queue_pair(io_queue_pair_id: &IoQueuePairId) -> usize;
}

#[derive(Debug, Clone, Copy)]
pub enum SysNvmeError {
	UnknownError = 0,
	ZeroPointerParameter = 1,
	DeviceDoesNotExist = 2,
	CouldNotIdentifyNamespaces = 3,
	NamespaceDoesNotExist = 4,
	MaxNumberOfQueuesReached = 5,
	CouldNotCreateIoQueuePair = 6,
	CouldNotDeleteIoQueuePair = 7,
	CouldNotFindIoQueuePair = 8,
	BufferIsZero = 9,
	BufferTooBig = 10,
	BufferIncorrectlySized = 11,
	CouldNotAllocateMemory = 12,
	CouldNotAllocateBuffer = 13,
	CouldNotDeallocateBuffer = 14,
	CouldNotReadFromIoQueuePair = 15,
	CouldNotWriteToIoQueuePair = 16,
	CouldNotClearNamespace = 17,
}

impl From<usize> for SysNvmeError {
	fn from(value: usize) -> Self {
		match value {
			1 => SysNvmeError::ZeroPointerParameter,
			2 => SysNvmeError::DeviceDoesNotExist,
			3 => SysNvmeError::CouldNotIdentifyNamespaces,
			4 => SysNvmeError::NamespaceDoesNotExist,
			5 => SysNvmeError::MaxNumberOfQueuesReached,
			6 => SysNvmeError::CouldNotCreateIoQueuePair,
			7 => SysNvmeError::CouldNotDeleteIoQueuePair,
			8 => SysNvmeError::CouldNotFindIoQueuePair,
			9 => SysNvmeError::BufferIsZero,
			10 => SysNvmeError::BufferTooBig,
			11 => SysNvmeError::BufferIncorrectlySized,
			12 => SysNvmeError::CouldNotAllocateMemory,
			13 => SysNvmeError::CouldNotAllocateBuffer,
			14 => SysNvmeError::CouldNotDeallocateBuffer,
			15 => SysNvmeError::CouldNotReadFromIoQueuePair,
			16 => SysNvmeError::CouldNotWriteToIoQueuePair,
			17 => SysNvmeError::CouldNotClearNamespace,
			_ => SysNvmeError::UnknownError,
		}
	}
}
