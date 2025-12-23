//! C-compatible math functions ([`math.h`]).
//!
//! [`math.h`]: https://en.cppreference.com/w/c/numeric/math

macro_rules! export {
    ($(fn $fn:ident($($arg:ident: $argty:ty),+) -> $retty:ty;)+) => {
        $(
            #[no_mangle]
            pub extern "C" fn $fn($($arg: $argty),+) -> $retty {
                ::libm::$fn($($arg),+)
            }
        )+
    };
}

export! {
	fn acos(x: f64) -> f64;
	fn acosf(x: f32) -> f32;
	fn acosh(x: f64) -> f64;
	fn acoshf(x: f32) -> f32;
	fn asin(x: f64) -> f64;
	fn asinf(x: f32) -> f32;
	fn asinh(x: f64) -> f64;
	fn asinhf(x: f32) -> f32;
	fn atan(x: f64) -> f64;
	fn atan2(y: f64, x: f64) -> f64;
	fn atan2f(y: f32, x: f32) -> f32;
	fn atanf(x: f32) -> f32;
	fn atanh(x: f64) -> f64;
	fn atanhf(x: f32) -> f32;
	fn cbrt(x: f64) -> f64;
	fn cbrtf(x: f32) -> f32;
	fn ceil(x: f64) -> f64;
	fn ceilf(x: f32) -> f32;
	fn copysign(x: f64, y: f64) -> f64;
	fn copysignf(x: f32, y: f32) -> f32;
	fn cos(x: f64) -> f64;
	fn cosf(x: f32) -> f32;
	fn cosh(x: f64) -> f64;
	fn coshf(x: f32) -> f32;
	fn erf(x: f64) -> f64;
	fn erfc(x: f64) -> f64;
	fn erfcf(x: f32) -> f32;
	fn erff(x: f32) -> f32;
	fn exp(x: f64) -> f64;
	fn exp10(x: f64) -> f64;
	fn exp10f(x: f32) -> f32;
	fn exp2(x: f64) -> f64;
	fn exp2f(x: f32) -> f32;
	fn expf(x: f32) -> f32;
	fn expm1(x: f64) -> f64;
	fn expm1f(x: f32) -> f32;
	fn fabs(n: f64) -> f64;
	fn fabsf(n: f32) -> f32;
	fn fdim(x: f64, y: f64) -> f64;
	fn fdimf(x: f32, y: f32) -> f32;
	fn floor(x: f64) -> f64;
	fn floorf(x: f32) -> f32;
	fn fma(x: f64, y: f64, z: f64) -> f64;
	fn fmaf(x: f32, y: f32, z: f32) -> f32;
	fn fmax(x: f64, y: f64) -> f64;
	fn fmaxf(x: f32, y: f32) -> f32;
	fn fmin(x: f64, y: f64) -> f64;
	fn fminf(x: f32, y: f32) -> f32;
	fn fmod(x: f64, y: f64) -> f64;
	fn fmodf(x: f32, y: f32) -> f32;
	// fn frexp(x: f64, n: &mut i32) -> f64;
	// fn frexpf(x: f32, n: &mut i32) -> f32;
	fn hypot(x: f64, y: f64) -> f64;
	fn hypotf(x: f32, y: f32) -> f32;
	fn ilogb(x: f64) -> i32;
	fn ilogbf(x: f32) -> i32;
	fn j0(x: f64) -> f64;
	fn j0f(x: f32) -> f32;
	fn j1(x: f64) -> f64;
	fn j1f(x: f32) -> f32;
	fn jn(n: i32, x: f64) -> f64;
	fn jnf(n: i32, x: f32) -> f32;
	fn ldexp(x: f64, n: i32) -> f64;
	fn ldexpf(x: f32, n: i32) -> f32;
	fn lgamma(x: f64) -> f64;
	// fn lgamma_r(x: f64, n: &mut i32) -> f64;
	fn lgammaf(x: f32) -> f32;
	// fn lgammaf_r(x: f32, n: &mut i32) -> f32;
	fn log(x: f64) -> f64;
	fn log10(x: f64) -> f64;
	fn log10f(x: f32) -> f32;
	fn log1p(x: f64) -> f64;
	fn log1pf(x: f32) -> f32;
	fn log2(x: f64) -> f64;
	fn log2f(x: f32) -> f32;
	fn logf(x: f32) -> f32;
	// fn modf(x: f64, y: &mut f64) -> f64;
	// fn modff(x: f32, y: &mut f32) -> f32;
	fn nextafter(x: f64, y: f64) -> f64;
	fn nextafterf(x: f32, y: f32) -> f32;
	fn pow(x: f64, y: f64) -> f64;
	fn powf(x: f32, y: f32) -> f32;
	fn remainder(x: f64, y: f64) -> f64;
	fn remainderf(x: f32, y: f32) -> f32;
	// fn remquo(x: f64, y: f64, n: &mut i32) -> f64;
	// fn remquof(x: f32, y: f32, n: &mut i32) -> f32;
	fn rint(x: f64) -> f64;
	fn rintf(x: f32) -> f32;
	fn round(x: f64) -> f64;
	fn roundf(x: f32) -> f32;
	fn scalbn(x: f64, n: i32) -> f64;
	fn scalbnf(x: f32, n: i32) -> f32;
	fn sin(x: f64) -> f64;
	// fn sincos(x: f64, s: &mut f64, c: &mut f64);
	// fn sincosf(x: f32, s: &mut f32, c: &mut f32);
	fn sinf(x: f32) -> f32;
	fn sinh(x: f64) -> f64;
	fn sinhf(x: f32) -> f32;
	fn sqrt(x: f64) -> f64;
	fn sqrtf(x: f32) -> f32;
	fn tan(x: f64) -> f64;
	fn tanf(x: f32) -> f32;
	fn tanh(x: f64) -> f64;
	fn tanhf(x: f32) -> f32;
	fn tgamma(x: f64) -> f64;
	fn tgammaf(x: f32) -> f32;
	fn trunc(n: f64) -> f64;
	fn truncf(n: f32) -> f32;
	fn y0(x: f64) -> f64;
	fn y0f(n: f32) -> f32;
	fn y1(n: f64) -> f64;
	fn y1f(n: f32) -> f32;
	fn yn(n: i32, x: f64) -> f64;
	fn ynf(n: i32, x: f32) -> f32;
}

macro_rules! export_out_param {
    ($(fn $fn:ident($($arg:ident: $argty:ty),+; $out:ident: $outty:ty) -> $retty:ty;)+) => {
        $(
            #[no_mangle]
            pub extern "C" fn $fn($($arg: $argty),+, $out: $outty) -> $retty {
                let (ret, out) = ::libm::$fn($($arg),+);
                *$out = out;
                ret
            }
        )+
    };
}

export_out_param! {
	fn frexp(x: f64; n: &mut i32) -> f64;
	fn frexpf(x: f32; n: &mut i32) -> f32;
	fn lgamma_r(x: f64; n: &mut i32) -> f64;
	fn lgammaf_r(x: f32; n: &mut i32) -> f32;
	fn modf(x: f64; y: &mut f64) -> f64;
	fn modff(x: f32; y: &mut f32) -> f32;
	fn remquo(x: f64, y: f64; n: &mut i32) -> f64;
	fn remquof(x: f32, y: f32; n: &mut i32) -> f32;
}

#[no_mangle]
pub extern "C" fn sincos(x: f64, s: &mut f64, c: &mut f64) {
	(*s, *c) = libm::sincos(x);
}

#[no_mangle]
pub extern "C" fn sincosf(x: f32, s: &mut f32, c: &mut f32) {
	(*s, *c) = libm::sincosf(x);
}
