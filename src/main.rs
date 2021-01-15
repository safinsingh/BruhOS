#![no_std]
#![no_main]
#![feature(asm)]
#![feature(const_panic)]
#![feature(panic_info_message)]
#![feature(panic_internals)]
#![warn(clippy::all)]

mod arch;
mod stdio;

use arch::cpu;
use core::{
	cell::UnsafeCell,
	panic::{Location, PanicInfo},
};
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

impl StivaleInfo {
	fn inner(&self) -> &StivaleStructure {
		// SAFETY: safe assuming it's called after STIVALE_STRUCT is set
		// properly
		unsafe { self.0.get().as_ref().unwrap().as_ref().unwrap() }
	}
}

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
	kiprintln!(
		"Detected bootloader: {} @ {}",
		STIVALE_STRUCT.inner().bootloader_brand().unwrap(),
		STIVALE_STRUCT.inner().bootloader_version().unwrap(),
	);

	loop {
		cpu::wait_for_interrupt();
	}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
	static DEFAULT_LOCATION: Location =
		Location::internal_constructor("UNKNOWN", 0, 0);
	let location = info.location().unwrap_or(&DEFAULT_LOCATION);

	keprintln!(
		"Kernel panicked with: {:#?}\n            Panicked at: {} ({}, {})\n            With payload: \
		 {:#?}",
		info.message().unwrap_or(&format_args!("UNKNOWN")),
		location.file(),
		location.line(),
		location.column(),
		info.payload()
	);

	loop {
		cpu::wait_for_interrupt();
	}
}
