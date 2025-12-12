use core::arch::asm;

/// This macro can be used to call system functions from user-space
#[macro_export]
macro_rules! syscall {
	($arg0:expr) => {
		$crate::x86_64::syscall0($arg0 as u64)
	};

	($arg0:expr, $arg1:expr) => {
		$crate::x86_64::syscall1($arg0 as u64, $arg1 as u64)
	};

	($arg0:expr, $arg1:expr, $arg2:expr) => {
		$crate::x86_64::syscall2($arg0 as u64, $arg1 as u64, $arg2 as u64)
	};

	($arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr) => {
		$crate::x86_64::syscall3($arg0 as u64, $arg1 as u64, $arg2 as u64, $arg3 as u64)
	};

	($arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr) => {
		$crate::x86_64::syscall4(
			$arg0 as u64,
			$arg1 as u64,
			$arg2 as u64,
			$arg3 as u64,
			$arg4 as u64,
		)
	};

	($arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr, $arg5:expr) => {
		$crate::x86_64::syscall5(
			$arg0 as u64,
			$arg1 as u64,
			$arg2 as u64,
			$arg3 as u64,
			$arg4 as u64,
			$arg5 as u64,
		)
	};

	($arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr, $arg5:expr, $arg6:expr) => {
		$crate::x86_64::syscall6(
			$arg0 as u64,
			$arg1 as u64,
			$arg2 as u64,
			$arg3 as u64,
			$arg4 as u64,
			$arg5 as u64,
			$arg6 as u64,
		)
	};
}

#[allow(dead_code)]
#[inline]
pub(crate) fn syscall0(arg0: u64) -> u64 {
	let ret: u64;
	unsafe {
		asm!("syscall",
			inlateout("rax") arg0 => ret,
			lateout("rcx") _,
			lateout("r11") _,
			options(preserves_flags, nostack)
		);
	}
	ret
}

#[allow(dead_code)]
#[inline]
pub(crate) fn syscall1(arg0: u64, arg1: u64) -> u64 {
	let ret: u64;
	unsafe {
		asm!("syscall",
			inlateout("rax") arg0 => ret,
			in("rdi") arg1,
			lateout("rcx") _,
			lateout("r11") _,
			options(preserves_flags, nostack)
		);
	}
	ret
}

#[allow(dead_code)]
#[inline]
pub(crate) fn syscall2(arg0: u64, arg1: u64, arg2: u64) -> u64 {
	let ret: u64;
	unsafe {
		asm!("syscall",
			inlateout("rax") arg0 => ret,
			in("rdi") arg1,
			in("rsi") arg2,
			lateout("rcx") _,
			lateout("r11") _,
			options(preserves_flags, nostack)
		);
	}
	ret
}

#[allow(dead_code)]
#[inline]
pub(crate) fn syscall3(arg0: u64, arg1: u64, arg2: u64, arg3: u64) -> u64 {
	let ret: u64;
	unsafe {
		asm!("syscall",
			inlateout("rax") arg0 => ret,
			in("rdi") arg1,
			in("rsi") arg2,
			in("rdx") arg3,
			lateout("rcx") _,
			lateout("r11") _,
			options(preserves_flags, nostack)
		);
	}
	ret
}

#[allow(dead_code)]
#[inline]
pub(crate) fn syscall4(arg0: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64) -> u64 {
	let ret: u64;
	unsafe {
		asm!("syscall",
			inlateout("rax") arg0 => ret,
			in("rdi") arg1,
			in("rsi") arg2,
			in("rdx") arg3,
			in("r10") arg4,
			lateout("rcx") _,
			lateout("r11") _,
			options(preserves_flags, nostack)
		);
	}
	ret
}

#[allow(dead_code)]
#[inline]
pub(crate) fn syscall5(arg0: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64) -> u64 {
	let ret: u64;
	unsafe {
		asm!("syscall",
			inlateout("rax") arg0 => ret,
			in("rdi") arg1,
			in("rsi") arg2,
			in("rdx") arg3,
			in("r10") arg4,
			in("r8") arg5,
			lateout("rcx") _,
			lateout("r11") _,
			options(preserves_flags, nostack)
		);
	}
	ret
}

#[allow(dead_code)]
#[inline]
pub(crate) fn syscall6(
	arg0: u64,
	arg1: u64,
	arg2: u64,
	arg3: u64,
	arg4: u64,
	arg5: u64,
	arg6: u64,
) -> u64 {
	let ret: u64;
	unsafe {
		asm!("syscall",
			inlateout("rax") arg0 => ret,
			in("rdi") arg1,
			in("rsi") arg2,
			in("rdx") arg3,
			in("r10") arg4,
			in("r8") arg5,
			in("r9") arg6,
			lateout("rcx") _,
			lateout("r11") _,
			options(preserves_flags, nostack)
		);
	}
	ret
}
