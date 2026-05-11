use core::arch::asm;

/// This macro can be used to call system functions from user-space
#[macro_export]
macro_rules! syscall {
	($arg0:expr) => {
		$crate::syscall::aarch64::syscall0($arg0 as u64)
	};

	($arg0:expr, $arg1:expr) => {
		$crate::syscall::aarch64::syscall1($arg0 as u64, $arg1 as u64)
	};

	($arg0:expr, $arg1:expr, $arg2:expr) => {
		$crate::syscall::aarch64::syscall2($arg0 as u64, $arg1 as u64, $arg2 as u64)
	};

	($arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr) => {
		$crate::syscall::aarch64::syscall3($arg0 as u64, $arg1 as u64, $arg2 as u64, $arg3 as u64)
	};

	($arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr) => {
		$crate::syscall::aarch64::syscall4(
			$arg0 as u64,
			$arg1 as u64,
			$arg2 as u64,
			$arg3 as u64,
			$arg4 as u64,
		)
	};

	($arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr, $arg5:expr) => {
		$crate::syscall::aarch64::syscall5(
			$arg0 as u64,
			$arg1 as u64,
			$arg2 as u64,
			$arg3 as u64,
			$arg4 as u64,
			$arg5 as u64,
		)
	};

	($arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr, $arg5:expr, $arg6:expr) => {
		$crate::syscall::aarch64::syscall6(
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
		asm!(
			"svc #0",
			in("x8") arg0,
			lateout("x0") ret,
			options(preserves_flags)
		);
	}
	ret
}

#[allow(dead_code)]
#[inline]
pub(crate) fn syscall1(arg0: u64, arg1: u64) -> u64 {
	let ret: u64;
	unsafe {
		asm!(
			"svc #0",
			in("x8") arg0,
			inlateout("x0") arg1 => ret,
			options(preserves_flags)
		);
	}
	ret
}

#[allow(dead_code)]
#[inline]
pub(crate) fn syscall2(arg0: u64, arg1: u64, arg2: u64) -> u64 {
	let ret: u64;
	unsafe {
		asm!(
			"svc #0",
			in("x8") arg0,
			inlateout("x0") arg1 => ret,
			in("x1") arg2,
			options(preserves_flags)
		);
	}
	ret
}

#[allow(dead_code)]
#[inline]
pub(crate) fn syscall3(arg0: u64, arg1: u64, arg2: u64, arg3: u64) -> u64 {
	let ret: u64;
	unsafe {
		asm!(
			"svc #0",
			in("x8") arg0,
			inlateout("x0") arg1 => ret,
			in("x1") arg2,
			in("x2") arg3,
			options(preserves_flags)
		);
	}
	ret
}

#[allow(dead_code)]
#[inline]
pub(crate) fn syscall4(arg0: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64) -> u64 {
	let ret: u64;
	unsafe {
		asm!(
			"svc #0",
			in("x8") arg0,
			inlateout("x0") arg1 => ret,
			in("x1") arg2,
			in("x2") arg3,
			in("x3") arg4,
			options(preserves_flags)
		);
	}
	ret
}

#[allow(dead_code)]
#[inline]
pub(crate) fn syscall5(arg0: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64) -> u64 {
	let ret: u64;
	unsafe {
		asm!(
			"svc #0",
			in("x8") arg0,
			inlateout("x0") arg1 => ret,
			in("x1") arg2,
			in("x2") arg3,
			in("x3") arg4,
			in("x4") arg5,
			options(preserves_flags)
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
		asm!(
			"svc #0",
			in("x8") arg0,
			inlateout("x0") arg1 => ret,
			in("x1") arg2,
			in("x2") arg3,
			in("x3") arg4,
			in("x4") arg5,
			in("x5") arg6,
			options(preserves_flags)
		);
	}
	ret
}
