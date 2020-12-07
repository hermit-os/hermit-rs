//extern crate compiler_builtins;
//use compiler_builtins::mem::{memcmp, memcpy, memmove, memset};

use std::hint::black_box;

pub fn memcpy_builtin(n: usize) {
    let v1 = vec![1u8; n];
    let mut v2 = vec![0u8; n];
    let src: &[u8] = black_box(&v1);
    let dst: &mut [u8] = black_box(&mut v2);
    
    dst.copy_from_slice(src);
}