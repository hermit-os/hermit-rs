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
pub extern "C" fn asin(n: f64) -> f64 {
	libm::asin(n)
}

#[no_mangle]
pub extern "C" fn asinf(n: f32) -> f32 {
	libm::asinf(n)
}

#[no_mangle]
pub extern "C" fn atan(n: f64) -> f64 {
	libm::atan(n)
}

#[no_mangle]
pub extern "C" fn atan2(a: f64, b: f64) -> f64 {
	libm::atan2(a, b)
}

pub extern "C" fn atan2f(a: f32, b: f32) -> f32 {
	libm::atan2f(a, b)
}

#[no_mangle]
pub extern "C" fn atanf(n: f32) -> f32 {
	libm::atanf(n)
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
pub extern "C" fn cosh(n: f64) -> f64 {
	libm::cosh(n)
}

#[no_mangle]
pub extern "C" fn coshf(n: f32) -> f32 {
	libm::coshf(n)
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
pub extern "C" fn expm1(n: f64) -> f64 {
	libm::expm1(n)
}

#[no_mangle]
pub extern "C" fn expm1f(n: f32) -> f32 {
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
pub extern "C" fn hypot(x: f64, y: f64) -> f64 {
	libm::hypot(x, y)
}

#[no_mangle]
pub extern "C" fn hypotf(x: f32, y: f32) -> f32 {
	libm::hypotf(x, y)
}

#[no_mangle]
pub extern "C" fn log(x: f64) -> f64 {
	libm::log(x)
}

pub fn logf(x: f32) -> f32 {
	libm::logf(x)
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
pub extern "C" fn sinh(n: f64) -> f64 {
	libm::sinh(n)
}

#[no_mangle]
pub extern "C" fn sinhf(n: f32) -> f32 {
	libm::sinhf(n)
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
