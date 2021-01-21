#![no_std]
#![no_main]
#![feature(asm)]
#![feature(const_panic)]
#![feature(panic_info_message)]
#![feature(panic_internals)]
#![feature(once_cell)]
#![feature(const_option)]
#![feature(const_raw_ptr_deref)]
#![deny(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(clippy::cargo)]

//! BruhOS is an x86_64 operating system

mod arch;
mod boot;
mod mm;
mod stdio;
mod util;

use arch::cpu;
use boot::STIVALE_STRUCT;
use core::panic::{Location, PanicInfo};
use mm::pmm;
use stdio::framebuffer::CommonColors;

/// Bootloader entrypoint (kernel main)
#[no_mangle]
pub extern "C" fn kmain(stivale_struct_ptr: usize) -> ! {
	// SAFETY:
	// 1. kmain lifetime is that of the entire program, so assigning an unsafe
	// cell to a pointer to one of its stack values is okay
	// 2. loading is valid when a stivale2-compliant bootloader is in use. WILL
	// cause UB otherwise.
	unsafe {
		STIVALE_STRUCT.set(stivale::load(stivale_struct_ptr));
	}

	pmm::init();
	pmm::sanity_check();

	kprintln!(include_str!("../res/ascii.txt"));
	ksprintln!("Everything works!");

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
		"Kernel panicked!\n\tWhere: {} ({}, {})\n\tWith: {:#?}",
		location.file(),
		location.line(),
		location.column(),
		info.message().unwrap_or(&format_args!("UNKNOWN")),
	);

	loop {
		cpu::wait_for_interrupt();
	}
}
