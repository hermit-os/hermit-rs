use core::alloc::Layout;
use std::ptr::{null_mut, NonNull};
use std::sync::atomic::{AtomicPtr, Ordering};

use align_address::Align;
use generic_once_cell::Lazy;
use spinning_top::RawSpinlock;
use talc::source::Claim;
use talc::TalcLock;

const MIN_SIZE: usize = 1 * 1024 * 1024;

static HEAP_END: AtomicPtr<u8> = AtomicPtr::new(null_mut());
static ALLOC: Lazy<RawSpinlock, TalcLock<RawSpinlock, Claim>> = Lazy::new(|| {
	let mut mem: *mut u8 = null_mut();

	if unsafe {
		crate::syscall::mmap(
			MIN_SIZE,
			hermit_abi::PROT_READ | hermit_abi::PROT_WRITE,
			&mut mem,
		)
	} == 0
	{
		unsafe { HEAP_END.store(mem.offset(MIN_SIZE.try_into().unwrap()), Ordering::Relaxed) };
		TalcLock::new(unsafe { Claim::new(mem, MIN_SIZE) })
	} else {
		panic!("Unable to initialize heap!");
	}
});

#[no_mangle]
pub extern "C" fn sys_malloc(size: usize, align: usize) -> *mut u8 {
	let layout = Layout::from_size_align(size, align).unwrap();
	unsafe {
		if let Some(mut addr) = ALLOC.lock().allocate(layout) {
			return addr.as_mut();
		}
		
		let heap_end = HEAP_END.load(Ordering::Acquire);
		let extend_size = size.align_up(MIN_SIZE);
		let mut result: *mut u8 = heap_end;

		if crate::syscall::mmap(
			extend_size,
			hermit_abi::PROT_READ | hermit_abi::PROT_WRITE,
			&mut result,
		) == 0
		{
			let mut new_heap_end = ALLOC.lock().extend(
				NonNull::new_unchecked(heap_end),
				heap_end.offset(extend_size.try_into().unwrap()),
			);
			HEAP_END.store(new_heap_end.as_mut(), Ordering::Release);

			if let Some(mut addr) = ALLOC.lock().allocate(layout) {
				return addr.as_mut();
			}
		}

		std::ptr::null_mut()
	}
}

#[no_mangle]
pub extern "C" fn sys_realloc(ptr: *mut u8, size: usize, align: usize, new_size: usize) -> *mut u8 {
	unsafe {
		let layout = Layout::from_size_align(size, align).unwrap();
		if new_size > size {
			if ALLOC.lock().try_grow_in_place(ptr, layout, new_size) {
				ptr
			} else {
				std::ptr::null_mut()
			}
		} else {
			ALLOC.lock().shrink(ptr, layout, new_size);
			ptr
		}
	}
}

#[no_mangle]
pub extern "C" fn sys_free(ptr: *mut u8, size: usize, align: usize) {
	let layout = Layout::from_size_align(size, align).unwrap();
	unsafe {
		ALLOC.lock().deallocate(ptr, layout);
	}
}
