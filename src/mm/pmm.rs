use super::{HIGH_HALF_OFFSET, PAGE_SIZE};
use crate::{kiprintln, ksprintln, polyfill, STIVALE_STRUCT};
use core::cell::UnsafeCell;
use spin::Mutex;
use stivale::memory::MemoryMapEntryType;

struct PmmBitmap(UnsafeCell<Mutex<Option<usize>>>);
unsafe impl Send for PmmBitmap {}
unsafe impl Sync for PmmBitmap {}

impl PmmBitmap {
	unsafe fn set_bitmap_ptr(&self, to: usize) {
		*self.0.get().as_ref().unwrap().lock() = Some(to);
	}

	fn get_bitmap_ptr(&self) -> *mut u8 {
		unsafe { self.0.get().as_ref().unwrap().lock().unwrap() as *mut u8 }
	}

	// offset = page-aligned address / page size
	fn bitmap_reset_bit(&self, offset: usize) {
		unsafe {
			asm!(
				"btr {}, {}",
				in(reg) self.get_bitmap_ptr().add(HIGH_HALF_OFFSET),
				in(reg) offset,
				options(nostack)
			);
		}
	}

	// offset = page-aligned address / page size
	fn bitmap_set_bit(&self, offset: usize) {
		unsafe {
			asm!(
				"bts {}, {}",
				in(reg) self.get_bitmap_ptr().add(HIGH_HALF_OFFSET),
				in(reg) offset,
				options(nostack)
			);
		}
	}

	// offset = page-aligned address / page size
	fn bitmap_test_bit(&self, offset: usize) -> bool {
		let flags: u64;

		unsafe {
			asm!(
				"bt {}, {}",
				"pushf",
				"pop {}",
				in(reg) self.get_bitmap_ptr().add(HIGH_HALF_OFFSET),
				in(reg) offset,
				out(reg) flags
			);
		}

		flags & 1 == 1
	}
}

static BITMAP: PmmBitmap = PmmBitmap(UnsafeCell::new(Mutex::new(None)));

pub fn init() {
	let mmap_usable = STIVALE_STRUCT
		.inner()
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
	let bitmap_size = polyfill::div_up(top_page as usize, super::PAGE_SIZE) / 8;
	kiprintln!("PMM bitmap size: {} KiB", bitmap_size / 1024);

	let mut bitmap_entry = 0;
	for (idx, entry) in mmap_usable.clone().enumerate() {
		if entry.size() >= bitmap_size as u64 {
			unsafe {
				BITMAP.set_bitmap_ptr(entry.start_address() as usize);
				polyfill::memset(
					BITMAP.get_bitmap_ptr().add(HIGH_HALF_OFFSET),
					0xFF,
					bitmap_size,
				);
			}

			// Huge brain moment
			// <does something big brain>
			// eh nvm

			ksprintln!("Selected region #{} to place bitmap in!", idx + 1);
			bitmap_entry = idx;
			break;
		}
	}

	// consume because we don't need it anymore
	for (idx, entry) in mmap_usable.enumerate() {
		kiprintln!("Marking usable region #{} as free...", idx + 1);
		let mut size = entry.size();
		let mut addr = entry.start_address();

		if idx == bitmap_entry {
			// omit the bitmap entry from free aspace
			kiprintln!("Omitting bitmap from free space...");
			size -= bitmap_size as u64;
			addr += bitmap_size as u64;
		}

		for bit in (0..size).step_by(PAGE_SIZE) {
			let bit = (addr + bit) as usize / PAGE_SIZE;
			BITMAP.bitmap_reset_bit(bit);
		}
	}

	kiprintln!("Initialized PMM bitmap at: {:p}", unsafe {
		BITMAP.get_bitmap_ptr().add(HIGH_HALF_OFFSET)
	});
}

pub fn sanity_check() {
	assert!(
		!BITMAP.bitmap_test_bit(unsafe {
			(BITMAP.get_bitmap_ptr().add(HIGH_HALF_OFFSET) as usize) / PAGE_SIZE
		} as usize),
		"Address space with bitmap marked as free!"
	);

	// ...

	ksprintln!("PMM sanity checks passed!");
}
