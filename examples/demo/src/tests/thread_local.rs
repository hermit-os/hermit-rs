pub fn test_thread_local() -> Result<(), ()> {
	#[repr(align(0x10))]
	struct Aligned(u8);

	thread_local! {
		static THREAD_LOCAL: Aligned = const { Aligned(0x42) };
	}

	THREAD_LOCAL.with(|thread_local| {
		assert_eq!(0x42, thread_local.0);
	});

	Ok(())
}
