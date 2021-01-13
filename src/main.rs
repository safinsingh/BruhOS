#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use core::panic::PanicInfo;
use stivale::StivaleHeader;

static STACK: [u8; 4096] = [0; 4096];
#[link_section = ".stivale2hdr"]
#[used]
static STIVALE_HDR: StivaleHeader = StivaleHeader::new(&STACK[0] as *const u8);

/// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
	loop {}
}

static HELLO: &[u8] = b"Hello World!";
#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start() -> ! {
	// this function is the entry point, since the linker looks for a function
	// named `_start` by default
	let vga_buffer = 0xb8000 as *mut u8;

	for (i, &byte) in HELLO.iter().enumerate() {
		unsafe {
			*vga_buffer.offset(i as isize * 2) = byte;
			*vga_buffer.offset(i as isize * 2 + 1) = 0xb;
		}
	}

	loop {}
}

// fn kmain(stivale_struct_ptr: usize) {
//     let stivale_struct = unsafe { stivale::load(stivale_struct_ptr) };
// }
