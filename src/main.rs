#![no_std]
#![no_main]
#![feature(asm)]
#![feature(const_panic)]
#![warn(clippy::all)]

mod arch;
mod stdio;

use arch::cpu;
use core::{cell::UnsafeCell, panic::PanicInfo};
use stivale::{HeaderFramebufferTag, StivaleHeader, StivaleStructure};

#[link_section = ".stivale2hdr"]
#[used]
static STIVALE_HDR: StivaleHeader = StivaleHeader::new(STACK[0] as *const u8)
	.tags((&FRAMEBUFFER_TAG as *const HeaderFramebufferTag).cast());

static STACK: [u8; 4096] = [0; 4096];
static FRAMEBUFFER_TAG: HeaderFramebufferTag =
	HeaderFramebufferTag::new().bpp(32);

struct StivaleInfo(UnsafeCell<Option<StivaleStructure>>);
unsafe impl Send for StivaleInfo {}
unsafe impl Sync for StivaleInfo {}

static STIVALE_STRUCT: StivaleInfo = StivaleInfo(UnsafeCell::new(None));

#[no_mangle]
pub fn kmain(stivale_struct_ptr: usize) -> ! {
	// SAFETY:
	// 1. kmain lifetime is that of the entire program, so assigning an unsafe
	// cell to a pointer to one of its stack values is okay
	// 2. loading is valid when a stivale2-compliant bootloader is in use. WILL
	// cause UB otherwise.
	unsafe {
		*STIVALE_STRUCT.0.get() = Some(stivale::load(stivale_struct_ptr));
	}

	kiprintln!("Loaded kernel");

	loop {
		cpu::wait_for_interrupt();
	}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
	loop {
		cpu::wait_for_interrupt();
	}
}
