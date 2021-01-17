use super::{HIGH_HALF_OFFSET, PAGE_SIZE};
use crate::{kiprintln, ksprintln, polyfill, STIVALE_STRUCT};
use core::cell::UnsafeCell;
use stivale::memory::MemoryMapEntryType;

struct PmmBitmap(UnsafeCell<Option<*mut u8>>);
unsafe impl Send for PmmBitmap {}
unsafe impl Sync for PmmBitmap {}

impl PmmBitmap {
	unsafe fn set(&self, to: *mut u8) {
		*self.0.get() = Some(to);
	}

	fn inner_mut(&self) -> *mut u8 {
		unsafe { self.0.get().as_ref().unwrap().unwrap() }
	}
}

static BITMAP: PmmBitmap = PmmBitmap(UnsafeCell::new(None));

pub fn init() {
	let mmap_usable = STIVALE_STRUCT
		.inner_mut()
		.memory_map()
		.unwrap()
		.iter()
		.filter(|e| matches!(e.entry_type(), MemoryMapEntryType::Usable));

	let mut top_page = 0;
	for entry in mmap_usable.clone() {
		ksprintln!(
			"Found usable memory map entry!\n\tstart_addr: {:#p}\n\tsize: {} \
			 MiB",
			entry.start_address() as *mut u8,
			entry.size() / 1024 / 1024
		);

		if entry.end_address() > top_page {
			top_page = entry.end_address();
		}
	}

	// highest page / page size / 8 bits per byte
	let bitmap_size = super::div_up(top_page as usize, super::PAGE_SIZE) / 8;
	kiprintln!("PMM bitmap size: {} KiB", bitmap_size / 1024);

	let mut bitmap_entry = 0;
	for (idx, entry) in mmap_usable.clone().enumerate() {
		if entry.size() >= bitmap_size as u64 {
			unsafe {
				BITMAP.set(entry.start_address() as *mut u8);
				polyfill::memset(
					BITMAP.inner_mut().add(HIGH_HALF_OFFSET),
					0xFF,
					bitmap_size,
				);

				// Huge brain moment
				// <does something big brain>
				bitmap_entry = idx;

				break;
			}
		}
	}

	// consume because we don't need it anymore
	for (idx, entry) in mmap_usable.enumerate() {
		kiprintln!("Marking usable region #{} as free...", idx + 1);
		let mut size = entry.size();
		let mut addr = entry.start_address();

		if idx == bitmap_entry {
			size -= bitmap_size as u64;
			addr += bitmap_size as u64;
		}

		for bit in (0..size).step_by(PAGE_SIZE) {
			let bit = (addr + bit) as usize / PAGE_SIZE;

			unsafe {
				asm!("btr {}, {}", in(reg) BITMAP.inner_mut(), in(reg) bit, options(nostack));
			}
		}
	}

	kiprintln!("Initialized PMM bitmap at: {:p}", unsafe {
		BITMAP.inner_mut().add(HIGH_HALF_OFFSET)
	});
}
