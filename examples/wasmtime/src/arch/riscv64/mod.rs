use core::arch::global_asm;

global_asm!(include_str!("setjmp.s"));
global_asm!(include_str!("longjmp.s"));
