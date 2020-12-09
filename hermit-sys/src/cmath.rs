#![allow(dead_code)]

#[no_mangle]
pub extern "C" fn acos(n: f64) -> f64 {
	libm::acos(n)
}

#[no_mangle]
pub extern "C" fn acosf(n: f32) -> f32 {
	libm::acosf(n)
}

#[no_mangle]
pub extern "C" fn acosh(x: f64) -> f64 {
	libm::acosh(x)
}

#[no_mangle]
pub extern "C" fn acoshf(x: f32) -> f32 {
	libm::acoshf(x)
}

#[no_mangle]
pub extern "C" fn asin(n: f64) -> f64 {
	libm::asin(n)
}

#[no_mangle]
pub extern "C" fn asinf(n: f32) -> f32 {
	libm::asinf(n)
}

#[no_mangle]
pub extern "C" fn asinh(x: f64) -> f64 {
	libm::asinh(x)
}

#[no_mangle]
pub extern "C" fn asinhf(x: f32) -> f32 {
	libm::asinhf(x)
}

#[no_mangle]
pub extern "C" fn atan(n: f64) -> f64 {
	libm::atan(n)
}

#[no_mangle]
pub extern "C" fn atan2(a: f64, b: f64) -> f64 {
	libm::atan2(a, b)
}

#[no_mangle]
pub extern "C" fn atan2f(a: f32, b: f32) -> f32 {
	libm::atan2f(a, b)
}

#[no_mangle]
pub extern "C" fn atanf(n: f32) -> f32 {
	libm::atanf(n)
}

#[no_mangle]
pub extern "C" fn atanh(x: f64) -> f64 {
	libm::atanh(x)
}

#[no_mangle]
pub extern "C" fn atanhf(x: f32) -> f32 {
	libm::atanhf(x)
}

#[no_mangle]
pub extern "C" fn cbrt(n: f64) -> f64 {
	libm::cbrt(n)
}

#[no_mangle]
pub extern "C" fn cbrtf(n: f32) -> f32 {
	libm::cbrtf(n)
}

#[no_mangle]
pub extern "C" fn ceil(x: f64) -> f64 {
	libm::ceil(x)
}

#[no_mangle]
pub extern "C" fn ceilf(x: f32) -> f32 {
	libm::ceilf(x)
}

#[no_mangle]
pub extern "C" fn copysign(x: f64, y: f64) -> f64 {
	libm::copysign(x, y)
}

#[no_mangle]
pub extern "C" fn copysignf(x: f32, y: f32) -> f32 {
	libm::copysignf(x, y)
}

#[no_mangle]
pub extern "C" fn cos(x: f64) -> f64 {
	libm::cos(x)
}

#[no_mangle]
pub extern "C" fn cosf(x: f32) -> f32 {
	libm::cosf(x)
}

#[no_mangle]
pub extern "C" fn cosh(x: f64) -> f64 {
	libm::cosh(x)
}

#[no_mangle]
pub extern "C" fn coshf(x: f32) -> f32 {
	libm::coshf(x)
}

#[no_mangle]
pub extern "C" fn erf(x: f64) -> f64 {
	libm::erf(x)
}

#[no_mangle]
pub extern "C" fn erfc(x: f64) -> f64 {
	libm::erfc(x)
}

#[no_mangle]
pub extern "C" fn erff(x: f32) -> f32 {
	libm::erff(x)
}

#[no_mangle]
pub extern "C" fn erfcf(x: f32) -> f32 {
	libm::erfcf(x)
}

#[no_mangle]
pub extern "C" fn exp(x: f64) -> f64 {
	libm::exp(x)
}

#[no_mangle]
pub extern "C" fn expf(x: f32) -> f32 {
	libm::expf(x)
}

#[no_mangle]
pub extern "C" fn exp2(x: f64) -> f64 {
	libm::exp2(x)
}

#[no_mangle]
pub extern "C" fn exp2f(x: f32) -> f32 {
	libm::exp2f(x)
}

#[no_mangle]
pub extern "C" fn exp10(x: f64) -> f64 {
	libm::exp10(x)
}

#[no_mangle]
pub extern "C" fn exp10f(x: f32) -> f32 {
	libm::exp10f(x)
}

#[no_mangle]
pub extern "C" fn expm1(n: f64) -> f64 {
	libm::expm1(n)
}

#[no_mangle]
pub extern "C" fn expm1f(n: f32) -> f32 {
	libm::expm1f(n)
}

#[no_mangle]
pub extern "C" fn fabs(n: f64) -> f64 {
	libm::fabs(n)
}

#[no_mangle]
pub extern "C" fn fabsf(n: f32) -> f32 {
	libm::expm1f(n)
}

#[no_mangle]
pub extern "C" fn fdim(a: f64, b: f64) -> f64 {
	libm::fdim(a, b)
}

#[no_mangle]
pub extern "C" fn fdimf(a: f32, b: f32) -> f32 {
	libm::fdimf(a, b)
}

#[no_mangle]
pub extern "C" fn floorf(x: f32) -> f32 {
	libm::floorf(x)
}

#[no_mangle]
pub extern "C" fn fma(x: f64, y: f64, z: f64) -> f64 {
	libm::fma(x, y, z)
}

#[no_mangle]
pub extern "C" fn fmaf(x: f32, y: f32, z: f32) -> f32 {
	libm::fmaf(x, y, z)
}

#[no_mangle]
pub extern "C" fn fmax(x: f64, y: f64) -> f64 {
	libm::fmax(x, y)
}

#[no_mangle]
pub extern "C" fn fmaxf(x: f32, y: f32) -> f32 {
	libm::fmaxf(x, y)
}

#[no_mangle]
pub extern "C" fn fmin(x: f64, y: f64) -> f64 {
	libm::fmin(x, y)
}

#[no_mangle]
pub extern "C" fn fminf(x: f32, y: f32) -> f32 {
	libm::fminf(x, y)
}

#[no_mangle]
pub extern "C" fn fmod(x: f64, y: f64) -> f64 {
	libm::fmod(x, y)
}

#[no_mangle]
pub extern "C" fn fmodf(x: f32, y: f32) -> f32 {
	libm::fmodf(x, y)
}

#[no_mangle]
pub extern "C" fn frexp(arg: f64, exp: *mut i32) -> f64 {
	let (mantissa, exponent) = libm::frexp(arg);
	unsafe {
		*exp = exponent;
	}
	mantissa
}

#[no_mangle]
pub extern "C" fn frexpf(arg: f32, exp: *mut i32) -> f32 {
	let (mantissa, exponent) = libm::frexpf(arg);
	unsafe {
		*exp = exponent;
	}
	mantissa
}

#[no_mangle]
pub extern "C" fn hypot(x: f64, y: f64) -> f64 {
	libm::hypot(x, y)
}

#[no_mangle]
pub extern "C" fn hypotf(x: f32, y: f32) -> f32 {
	libm::hypotf(x, y)
}

#[no_mangle]
pub extern "C" fn ilogb(x: f64) -> i32 {
	libm::ilogb(x)
}

#[no_mangle]
pub extern "C" fn ilogbf(x: f32) -> i32 {
	libm::ilogbf(x)
}

#[no_mangle]
pub extern "C" fn j0(x: f64) -> f64 {
	libm::j0(x)
}

#[no_mangle]
pub extern "C" fn j0f(x: f32) -> f32 {
	libm::j0f(x)
}

#[no_mangle]
pub extern "C" fn j1(x: f64) -> f64 {
	libm::j1(x)
}

#[no_mangle]
pub extern "C" fn j1f(x: f32) -> f32 {
	libm::j1f(x)
}

#[no_mangle]
pub extern "C" fn jn(n: i32, x: f64) -> f64 {
	libm::jn(n, x)
}

#[no_mangle]
pub extern "C" fn jnf(n: i32, x: f32) -> f32 {
	libm::jnf(n, x)
}

#[no_mangle]
pub extern "C" fn ldexp(x: f64, n: i32) -> f64 {
	libm::ldexp(x, n)
}

#[no_mangle]
pub extern "C" fn ldexpf(x: f32, n: i32) -> f32 {
	libm::ldexpf(x, n)
}

#[no_mangle]
pub extern "C" fn lgamma(x: f64) -> f64 {
	libm::lgamma(x)
}

#[no_mangle]
pub extern "C" fn lgammaf(x: f32) -> f32 {
	libm::lgammaf(x)
}

#[no_mangle]
pub extern "C" fn lgamma_r(x: f64, signp: *mut i32) -> f64 {
	let (lgamma, r) = libm::lgamma_r(x);
	unsafe {
		*signp = r;
	}
	lgamma
}

#[no_mangle]
pub extern "C" fn lgammaf_r(x: f32, signp: *mut i32) -> f32 {
	let (lgamma, r) = libm::lgammaf_r(x);
	unsafe {
		*signp = r;
	}
	lgamma
}

#[no_mangle]
pub extern "C" fn log(x: f64) -> f64 {
	libm::log(x)
}

#[no_mangle]
pub extern "C" fn log10(x: f64) -> f64 {
	libm::log10(x)
}

#[no_mangle]
pub extern "C" fn log10f(x: f32) -> f32 {
	libm::log10f(x)
}

#[no_mangle]
pub extern "C" fn logf(x: f32) -> f32 {
	libm::logf(x)
}

#[no_mangle]
pub extern "C" fn log2(x: f64) -> f64 {
	libm::log2(x)
}

#[no_mangle]
pub extern "C" fn log2f(x: f32) -> f32 {
	libm::log2f(x)
}

#[no_mangle]
pub extern "C" fn log1p(n: f64) -> f64 {
	libm::log1p(n)
}

#[no_mangle]
pub extern "C" fn log1pf(n: f32) -> f32 {
	libm::log1pf(n)
}

#[no_mangle]
pub extern "C" fn modf(x: f64, integer: *mut f64) -> f64 {
	let (frac_part, int_part) = libm::modf(x);
	unsafe {
		*integer = int_part;
	}
	frac_part
}

#[no_mangle]
pub extern "C" fn modff(x: f32, integer: *mut f32) -> f32 {
	let (frac_part, int_part) = libm::modff(x);
	unsafe {
		*integer = int_part;
	}
	frac_part
}

#[no_mangle]
pub extern "C" fn nextafter(x: f64, y: f64) -> f64 {
	libm::nextafter(x, y)
}

#[no_mangle]
pub extern "C" fn nextafterf(x: f32, y: f32) -> f32 {
	libm::nextafterf(x, y)
}

#[no_mangle]
pub extern "C" fn pow(x: f64, y: f64) -> f64 {
	libm::pow(x, y)
}

#[no_mangle]
pub extern "C" fn powf(x: f32, y: f32) -> f32 {
	libm::powf(x, y)
}

#[no_mangle]
pub extern "C" fn remainder(x: f64, y: f64) -> f64 {
	libm::remainder(x, y)
}

#[no_mangle]
pub extern "C" fn remainderf(x: f32, y: f32) -> f32 {
	libm::remainderf(x, y)
}

#[no_mangle]
pub extern "C" fn remquo(x: f64, y: f64, quotient: *mut i32) -> f64 {
	let (rem, quo) = libm::remquo(x, y);
	unsafe {
		*quotient = quo;
	}
	rem
}

#[no_mangle]
pub extern "C" fn remquof(x: f32, y: f32, quotient: *mut i32) -> f32 {
	let (rem, quo) = libm::remquof(x, y);
	unsafe {
		*quotient = quo;
	}
	rem
}

#[no_mangle]
pub extern "C" fn round(n: f64) -> f64 {
	libm::round(n)
}

#[no_mangle]
pub extern "C" fn roundf(n: f32) -> f32 {
	libm::roundf(n)
}

#[no_mangle]
pub extern "C" fn scalbn(x: f64, n: i32) -> f64 {
	libm::scalbn(x, n)
}

#[no_mangle]
pub extern "C" fn scalbnf(x: f32, n: i32) -> f32 {
	libm::scalbnf(x, n)
}

#[no_mangle]
pub extern "C" fn sin(n: f64) -> f64 {
	libm::sin(n)
}

#[no_mangle]
pub extern "C" fn sinf(n: f32) -> f32 {
	libm::sinf(n)
}

#[no_mangle]
pub extern "C" fn sincos(n: f64, sin: *mut f64, cos: *mut f64) {
	let (res_sin, res_cos) = libm::sincos(n);
	unsafe {
		*sin = res_sin;
		*cos = res_cos;
	}
}

#[no_mangle]
pub extern "C" fn sincosf(n: f32, sin: *mut f32, cos: *mut f32) {
	let (res_sin, res_cos) = libm::sincosf(n);
	unsafe {
		*sin = res_sin;
		*cos = res_cos;
	}
}

#[no_mangle]
pub extern "C" fn sinh(n: f64) -> f64 {
	libm::sinh(n)
}

#[no_mangle]
pub extern "C" fn sinhf(n: f32) -> f32 {
	libm::sinhf(n)
}

#[no_mangle]
pub extern "C" fn sqrt(x: f64) -> f64 {
	libm::sqrt(x)
}

#[no_mangle]
pub extern "C" fn sqrtf(x: f32) -> f32 {
	libm::sqrtf(x)
}

#[no_mangle]
pub extern "C" fn tan(n: f64) -> f64 {
	libm::tan(n)
}

#[no_mangle]
pub extern "C" fn tanf(n: f32) -> f32 {
	libm::tanf(n)
}

#[no_mangle]
pub extern "C" fn tanh(n: f64) -> f64 {
	libm::tanh(n)
}

#[no_mangle]
pub extern "C" fn tanhf(n: f32) -> f32 {
	libm::tanhf(n)
}

#[no_mangle]
pub extern "C" fn tgamma(n: f64) -> f64 {
	libm::tgamma(n)
}

#[no_mangle]
pub extern "C" fn tgammaf(n: f32) -> f32 {
	libm::tgammaf(n)
}

#[no_mangle]
pub extern "C" fn trunc(n: f64) -> f64 {
	libm::trunc(n)
}

#[no_mangle]
pub extern "C" fn truncf(n: f32) -> f32 {
	libm::truncf(n)
}

#[no_mangle]
pub extern "C" fn y0(n: f64) -> f64 {
	libm::y0(n)
}

#[no_mangle]
pub extern "C" fn y0f(n: f32) -> f32 {
	libm::y0f(n)
}

#[no_mangle]
pub extern "C" fn y1(n: f64) -> f64 {
	libm::y1(n)
}

#[no_mangle]
pub extern "C" fn y1f(n: f32) -> f32 {
	libm::y1f(n)
}

#[no_mangle]
pub extern "C" fn yn(n: i32, x: f64) -> f64 {
	libm::yn(n, x)
}

#[no_mangle]
pub extern "C" fn ynf(n: i32, x: f32) -> f32 {
	libm::ynf(n, x)
}
