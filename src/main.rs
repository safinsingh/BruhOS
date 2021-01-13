#![no_std]
#![no_main]

use core::panic::PanicInfo;
use stivale::StivaleHeader;

static STACK: [u8; 4096] = [0; 4096];
#[link_section = ".stivale2hdr"]
#[used]
static STIVALE_HDR: StivaleHeader = StivaleHeader::new(STACK[0] as *const u8);

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
	loop {}
}

#[no_mangle]
pub extern "C" fn kmain(stivale_struct_ptr: usize) -> ! {
	let stivale_struct = unsafe { stivale::load(stivale_struct_ptr) };
	let vga_buffer = 0xb8000 as *mut u8;

	for (i, &byte) in stivale_struct
		.bootloader_brand()
		.unwrap()
		.as_bytes()
		.iter()
		.enumerate()
	{
		unsafe {
			*vga_buffer.offset(i as isize * 2) = byte;
			*vga_buffer.offset(i as isize * 2 + 1) = 0xb;
		}
	}

	loop {}
}
