# The code is derived from the musl implementation
# of longjmp.
#
# Copyright 2011-2012 Nicholas J. Kain,
# licensed under standard MIT license
.section .text
.global longjmp
longjmp:
	xor eax,eax
	cmp esi, 1              /* CF = val ? 0 : 1 */
	adc eax, esi            /* eax = val + !val */
	mov rbx, [rdi]
	mov rbp, [rdi+8] 
	mov r12, [rdi+16] 
	mov r13, [rdi+24] 
	mov r14, [rdi+32] 
	mov r15, [rdi+40] 
	mov rsp, [rdi+48] 
	jmp [rdi+56]