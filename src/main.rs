#![no_std]
#![no_main]
#![feature(asm)]
#![feature(const_panic)]
#![warn(clippy::all)]

mod arch;
mod stdio;

use arch::cpu;
use core::panic::PanicInfo;
use stdio::framebuffer::FramebufferWriter;
use stivale::{HeaderFramebufferTag, StivaleHeader};

static STACK: [u8; 4096] = [0; 4096];
static FRAMEBUFFER_TAG: HeaderFramebufferTag =
	HeaderFramebufferTag::new().bpp(32);

#[link_section = ".stivale2hdr"]
#[used]
static STIVALE_HDR: StivaleHeader = StivaleHeader::new(STACK[0] as *const u8)
	.tags((&FRAMEBUFFER_TAG as *const HeaderFramebufferTag).cast());

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
	loop {
		cpu::wait_for_interrupt();
	}
}

#[no_mangle]
pub fn kmain(stivale_struct_ptr: usize) -> ! {
	// SAFETY: valid when a stivale2-compliant bootloader is in use. WILL cause
	// UB otherwise.
	let stivale_struct = unsafe { stivale::load(stivale_struct_ptr) };

	let mut w = FramebufferWriter::new(stivale_struct.framebuffer().unwrap());
	kprint!(w, "say hello to custom font!");

	loop {
		cpu::wait_for_interrupt();
	}
}
