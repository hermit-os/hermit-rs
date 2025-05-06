# The code is derived from the musl implementation
# of setjmp.
#
# Copyright 2011-2012 Nicholas J. Kain,
# licensed under standard MIT license
.section .text
.global setjmp
setjmp:
mov [rdi], rbx
mov [rdi+8], rbp
mov [rdi+16], r12
mov [rdi+24], r13
mov [rdi+32], r14
mov [rdi+40], r15
lea rdx, [rsp+8] # rsp without current ret addr
mov [rdi+48], rdx
mov rdi, rsp     # save return addr ptr for new rip
mov [rdi+56], rdx
xor rax, rax
ret
