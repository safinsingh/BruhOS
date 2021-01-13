#![no_std]
#![no_main]
#![feature(asm)]
#![warn(clippy::all)]

mod arch;

use core::panic::PanicInfo;
use stivale::{HeaderFramebufferTag, StivaleHeader};

static STACK: [u8; 4096] = [0; 4096];
static FRAMEBUFFER_TAG: HeaderFramebufferTag =
	HeaderFramebufferTag::new().bpp(24);

#[link_section = ".stivale2hdr"]
#[used]
static STIVALE_HDR: StivaleHeader = StivaleHeader::new(STACK[0] as *const u8)
	.tags((&FRAMEBUFFER_TAG as *const HeaderFramebufferTag).cast());

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
	loop {
		arch::cpu::wait_for_interrupt();
	}
}

#[no_mangle]
pub fn kmain(stivale_struct_ptr: usize) -> ! {
	// SAFETY: valid when a stivale2-compliant bootloader is in use. May cause
	// UB otherwise.
	let stivale_struct = unsafe { stivale::load(stivale_struct_ptr) };

	let framebuffer = stivale_struct.framebuffer().unwrap();
	for i in 0..framebuffer.size() {
		unsafe { *((framebuffer.start_address() + i) as *mut u8) = 0xA0 }
	}

	loop {
		arch::cpu::wait_for_interrupt();
	}
}
