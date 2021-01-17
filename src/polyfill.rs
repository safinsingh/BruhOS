// Taken from: https://github.com/rust-lang/compiler-builtins/blob/master/src/mem/mod.rs

#[inline(always)]
pub unsafe fn copy_forward(dest: *mut u8, src: *const u8, count: usize) {
	let qword_count = count >> 3;
	let byte_count = count & 0b111;
	// FIXME: Use the Intel syntax once we drop LLVM 9 support on
	// rust-lang/rust.
	asm!(
		"repe movsq (%rsi), (%rdi)",
		"mov {byte_count:e}, %ecx",
		"repe movsb (%rsi), (%rdi)",
		byte_count = in(reg) byte_count,
		inout("rcx") qword_count => _,
		inout("rdi") dest => _,
		inout("rsi") src => _,
		options(att_syntax, nostack, preserves_flags)
	);
}

#[inline(always)]
pub unsafe fn copy_backward(dest: *mut u8, src: *const u8, count: usize) {
	let qword_count = count >> 3;
	let byte_count = count & 0b111;
	// FIXME: Use the Intel syntax once we drop LLVM 9 support on
	// rust-lang/rust.
	asm!(
		"std",
		"repe movsq (%rsi), (%rdi)",
		"movl {byte_count:e}, %ecx",
		"addq $7, %rdi",
		"addq $7, %rsi",
		"repe movsb (%rsi), (%rdi)",
		"cld",
		byte_count = in(reg) byte_count,
		inout("rcx") qword_count => _,
		inout("rdi") dest.add(count).wrapping_sub(8) => _,
		inout("rsi") src.add(count).wrapping_sub(8) => _,
		options(att_syntax, nostack)
	);
}

#[inline(always)]
pub unsafe fn set_bytes(s: *mut u8, c: u8, n: usize) {
	let mut i = 0;
	while i < n {
		*s.add(i) = c;
		i += 1;
	}
}

pub unsafe extern "C" fn memmove(
	dest: *mut u8,
	src: *const u8,
	n: usize,
) -> *mut u8 {
	let delta = (dest as usize).wrapping_sub(src as usize);
	if delta >= n {
		// We can copy forwards because either dest is far enough ahead of src,
		// or src is ahead of dest (and delta overflowed).
		copy_forward(dest, src, n);
	} else {
		copy_backward(dest, src, n);
	}
	dest
}

#[allow(warnings)]
#[cfg(not(target_pointer_width = "16"))]
type c_int = i32;
pub unsafe extern "C" fn memset(s: *mut u8, c: c_int, n: usize) -> *mut u8 {
	set_bytes(s, c as u8, n);
	s
}
