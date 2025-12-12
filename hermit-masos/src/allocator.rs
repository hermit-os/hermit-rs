use core::alloc::Layout;
use core::ptr::NonNull;
use std::mem::MaybeUninit;

use generic_once_cell::Lazy;
use spinning_top::RawSpinlock;
use talc::{ErrOnOom, Talc, Talck};

static ALLOC: Lazy<RawSpinlock, Talck<RawSpinlock, ErrOnOom>> = Lazy::new(|| {
	take_static::take_static! {
		static MEM: [MaybeUninit<u8>; 0x1000] = [MaybeUninit::uninit(); 0x1000];
	}

	let mem = MEM.take().unwrap();

	let mut talc = Talc::new(talc::ErrOnOom);
	unsafe {
		talc.claim(mem.into()).unwrap();
	}

	Talck::new(talc)
});

#[no_mangle]
pub extern "C" fn sys_malloc(size: usize, align: usize) -> *mut u8 {
	let layout = Layout::from_size_align(size, align).unwrap();
	unsafe { ALLOC.lock().malloc(layout).unwrap().as_mut() }
}

#[no_mangle]
pub extern "C" fn sys_realloc(ptr: *mut u8, size: usize, align: usize, new_size: usize) -> *mut u8 {
	unsafe {
		let layout = Layout::from_size_align(size, align).unwrap();
		if new_size > size {
			ALLOC
				.lock()
				.grow(NonNull::new_unchecked(ptr), layout, new_size)
				.unwrap()
				.as_mut()
		} else {
			ALLOC
				.lock()
				.shrink(NonNull::new_unchecked(ptr), layout, new_size);
			ptr
		}
	}
}

#[no_mangle]
pub extern "C" fn sys_free(ptr: *mut u8, size: usize, align: usize) {
	let layout = Layout::from_size_align(size, align).unwrap();
	unsafe {
		ALLOC.lock().free(NonNull::new_unchecked(ptr), layout);
	}
}
