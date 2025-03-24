//! Inspired by the Rust standard TLS tests:
//! <https://github.com/rust-lang/rust/tree/master/library/std/tests/thread_local>

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

#[cfg(target_os = "hermit")]
use hermit as _;

// --- Main TLS demonstration ---

// TLS variables of various types.
thread_local! {
	static TLS_VALUE: Cell<i32> = const { Cell::new(0) };
	static TLS_U64: Cell<u64> = const { Cell::new(0) };
	static TLS_F64: Cell<f64> = const { Cell::new(0.0) };
	static TLS_BOOL: Cell<bool> = const { Cell::new(false) };
	static TLS_CHAR: Cell<char> = const { Cell::new('A') };
	static TLS_STRING: Cell<&'static str> = const { Cell::new("Initial String") };
	static TLS_U8: Cell<u8> = const { Cell::new(0) };
	static TLS_U64_2: Cell<u64> = const { Cell::new(0xdeadbeef) };
}

// A custom type with 128-byte alignment.
#[derive(Clone, Copy)]
#[repr(align(128))]
struct AlignedType(u8);
thread_local! {
	static TLS_ALIGNED: Cell<AlignedType> = const { Cell::new(AlignedType(0x42)) };
}

// Global flag to detect TLS destructor execution.
static TLS_DESTRUCTOR_RAN: AtomicBool = AtomicBool::new(false);

// A type whose destructor sets the flag.
struct DtorNotifier;
impl Drop for DtorNotifier {
	fn drop(&mut self) {
		TLS_DESTRUCTOR_RAN.store(true, Ordering::SeqCst);
	}
}

fn main() {
	println!("Starting TLS demonstration");

	// Dynamically determine thread count: 2 x number of available cores.
	let num_threads = std::thread::available_parallelism()
		.map(|n| n.get() * 2)
		.unwrap_or(4);
	println!("Spawning {} threads (2 x number of cores)", num_threads);

	let mut handles = vec![];

	// Spawn threads to test TLS isolation and modification.
	for i in 0..num_threads {
		handles.push(thread::spawn(move || {
			println!("Thread {} started", i);

			// Check alignment.
			TLS_ALIGNED.with(|x| assert!(x.as_ptr().is_aligned()));

			// Set initial values.
			TLS_VALUE.set(i as i32);
			TLS_U64.set(i as u64);
			TLS_F64.set(i as f64);
			TLS_BOOL.set(i % 2 != 0);
			TLS_CHAR.set((65 + i as u8) as char); // A, B, C, D...
			TLS_STRING.set("String changed");
			TLS_U8.set(i as u8);
			TLS_ALIGNED.set(AlignedType(0x42 + i as u8));
			TLS_U64_2.set(0xdeadbeef + i as u64);

			println!("Thread {}: TLS_VALUE set to {}", i, TLS_VALUE.get());
			println!("Thread {}: TLS_U64 set to {}", i, TLS_U64.get());
			println!("Thread {}: TLS_F64 set to {}", i, TLS_F64.get());
			println!("Thread {}: TLS_BOOL set to {}", i, TLS_BOOL.get());
			println!("Thread {}: TLS_CHAR set to {}", i, TLS_CHAR.get());
			println!("Thread {}: TLS_STRING set to {}", i, TLS_STRING.get());
			println!("Thread {}: TLS_U8 set to {}", i, TLS_U8.get());
			println!("Thread {}: TLS_ALIGNED set to {}", i, TLS_ALIGNED.get().0);
			println!("Thread {}: TLS_U64_2 set to {}", i, TLS_U64_2.get());

			// Simulate work.
			thread::sleep(Duration::from_millis(100 * ((i as u64) + 1)));

			// Verify and modify values.
			assert_eq!(TLS_VALUE.get(), i as i32);
			TLS_VALUE.set(TLS_VALUE.get() + 10);
			println!("Thread {}: TLS_VALUE set to {}", i, TLS_VALUE.get());

			assert_eq!(TLS_U64.get(), i as u64);
			TLS_U64.set(TLS_U64.get() + 10);
			println!("Thread {}: TLS_U64 set to {}", i, TLS_U64.get());

			assert_eq!(TLS_F64.get(), i as f64);
			TLS_F64.set(TLS_F64.get() + 10.0);
			println!("Thread {}: TLS_F64 set to {}", i, TLS_F64.get());

			assert_eq!(TLS_BOOL.get(), (i % 2 != 0));
			TLS_BOOL.set(!TLS_BOOL.get());
			println!("Thread {}: TLS_BOOL set to {}", i, TLS_BOOL.get());

			assert_eq!(TLS_CHAR.get(), (65 + i as u8) as char);
			TLS_CHAR.set((97 + i as u8) as char);
			println!("Thread {}: TLS_CHAR set to {}", i, TLS_CHAR.get());

			assert_eq!(TLS_STRING.get(), "String changed");
			TLS_STRING.set("String changed again");
			println!("Thread {}: TLS_STRING set to {}", i, TLS_STRING.get());

			assert_eq!(TLS_U8.get(), i as u8);
			TLS_U8.set(TLS_U8.get() + 10);
			println!("Thread {}: TLS_U8 set to {}", i, TLS_U8.get());

			assert_eq!(TLS_ALIGNED.get().0, 0x42 + i as u8);
			TLS_ALIGNED.set(AlignedType(TLS_ALIGNED.get().0 + 10));
			println!("Thread {}: TLS_ALIGNED set to {}", i, TLS_ALIGNED.get().0);

			assert_eq!(TLS_U64_2.get(), 0xdeadbeef + i as u64);
			TLS_U64_2.set(TLS_U64_2.get() ^ 0xf0f0f0f0);
			println!("Thread {}: TLS_U64_2 set to {:#x}", i, TLS_U64_2.get());

			// Verify modified values.
			assert_eq!(TLS_VALUE.get(), i as i32 + 10);
			assert_eq!(TLS_U64.get(), i as u64 + 10);
			assert_eq!(TLS_F64.get(), i as f64 + 10.0);
			assert_eq!(TLS_BOOL.get(), (i % 2 == 0));
			assert_eq!(TLS_CHAR.get(), (97 + i as u8) as char);
			assert_eq!(TLS_STRING.get(), "String changed again");
			assert_eq!(TLS_U8.get(), i as u8 + 10);
			assert_eq!(TLS_ALIGNED.get().0, 0x42 + i as u8 + 10);
			assert_eq!(TLS_U64_2.get(), (0xdeadbeef + i as u64) ^ 0xf0f0f0f0);

			println!("Thread {} finished", i);
		}));
	}

	for handle in handles {
		handle.join().unwrap();
	}

	println!("TLS demonstration finished");

	// --- Additional TLS scenarios ---

	// 1. Computed initializer test.
	{
		fn square(i: i32) -> i32 {
			i * i
		}
		thread_local! {
			static COMPUTED_TLS: i32 = square(3);
		}
		COMPUTED_TLS.with(|val| {
			println!("Computed TLS value: {}", *val);
			assert_eq!(*val, 9);
		});
	}

	// 2. TLS with RefCell<HashMap>.
	{
		fn create_map() -> RefCell<HashMap<i32, i32>> {
			let mut m = HashMap::new();
			m.insert(1, 2);
			RefCell::new(m)
		}
		thread_local! {
			static TLS_MAP: RefCell<HashMap<i32, i32>> = create_map();
		}
		TLS_MAP.with(|map| {
			let value = map.borrow().get(&1).cloned().unwrap_or(0);
			println!("TLS_MAP value for key 1: {}", value);
			assert_eq!(value, 2);
		});
	}

	// 3. TLS with RefCell<Vec>.
	{
		thread_local! {
			static TLS_VEC: RefCell<Vec<u32>> = RefCell::new(vec![1, 2, 3]);
		}
		TLS_VEC.with(|vec| {
			println!("Initial TLS_VEC length: {}", vec.borrow().len());
			assert_eq!(vec.borrow().len(), 3);
			vec.borrow_mut().push(4);
			println!("TLS_VEC[3]: {}", vec.borrow()[3]);
			assert_eq!(vec.borrow()[3], 4);
		});
	}

	// 4. TLS destructor test.
	{
		thread_local! {
			static TLS_DTOR: DtorNotifier = const { DtorNotifier };
		}
		let handle = thread::spawn(|| {
			TLS_DTOR.with(|_| {
				println!("Thread: TLS_DTOR set");
			});
		});
		handle.join().unwrap();
		thread::sleep(Duration::from_millis(50));
		let flag_val = TLS_DESTRUCTOR_RAN.load(Ordering::SeqCst);
		println!("TLS destructor flag: {}", flag_val);
		assert!(flag_val, "TLS destructor did not run");
	}

	println!("Additional TLS scenarios finished");
}
