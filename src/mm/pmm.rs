use super::PAGE_SIZE;
use crate::{kiprintln, ksprintln, polyfill, STIVALE_STRUCT};
use core::{
	alloc::{GlobalAlloc, Layout},
	cell::UnsafeCell,
};
use spin::Mutex;
use stivale::memory::MemoryMapEntryType;

struct PmmInner {
	bitmap_ptr: Option<usize>,
	highest_bit: Option<usize>,
	last_used_page: usize,
}

struct Pmm(UnsafeCell<Mutex<PmmInner>>);
unsafe impl Send for Pmm {}
unsafe impl Sync for Pmm {}

impl Pmm {
	const fn new() -> Self {
		Self(UnsafeCell::new(Mutex::new(PmmInner {
			bitmap_ptr: None,
			highest_bit: None,
			last_used_page: 0,
		})))
	}

	#[inline]
	unsafe fn set_bitmap_ptr(&self, to: usize) {
		self.0.get().as_ref().unwrap().lock().bitmap_ptr = Some(to);
	}

	#[inline]
	unsafe fn set_highest_bit(&self, to: usize) {
		self.0.get().as_ref().unwrap().lock().highest_bit = Some(to);
	}

	#[inline]
	unsafe fn set_last_used_page(&self, to: usize) {
		self.0.get().as_ref().unwrap().lock().last_used_page = to;
	}

	#[inline]
	fn get_bitmap_ptr(&self) -> *mut u8 {
		unsafe {
			self.0.get().as_ref().unwrap().lock().bitmap_ptr.unwrap() as *mut u8
		}
	}

	#[inline]
	fn get_highest_bit(&self) -> usize {
		unsafe { self.0.get().as_ref().unwrap().lock().highest_bit.unwrap() }
	}

	#[inline]
	fn get_last_used_page(&self) -> usize {
		unsafe { self.0.get().as_ref().unwrap().lock().last_used_page }
	}

	// offset = page-aligned address / page size
	fn bitmap_reset_bit(&self, offset: usize) {
		unsafe {
			*self.get_bitmap_ptr().add(polyfill::div_up(offset, 8)) &=
				0 << (8 - (offset % 8) - 1);
		}
	}

	// offset = page-aligned address / page size
	fn bitmap_set_bit(&self, offset: usize) {
		unsafe {
			*self.get_bitmap_ptr().add(polyfill::div_up(offset, 8)) |=
				1 << (8 - (offset % 8) - 1);
		}
	}

	// offset = page-aligned address / page size
	fn bitmap_test_bit(&self, offset: usize) -> bool {
		unsafe {
			(*self.get_bitmap_ptr().add(polyfill::div_up(offset, 8))
				>> (8 - (offset % 8) - 1))
				& 1 == 1
		}
	}
}

static PMM: Pmm = Pmm::new();

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
	let bitmap_bits = polyfill::div_up(top_page as usize, super::PAGE_SIZE);
	unsafe {
		PMM.set_highest_bit(bitmap_bits);
	}
	let bitmap_size = bitmap_bits / 8;
	kiprintln!("PMM bitmap size: {} KiB", bitmap_size / 1024);

	let mut bitmap_entry = 0;
	for (idx, entry) in mmap_usable.clone().enumerate() {
		if entry.size() >= bitmap_size as u64 {
			unsafe {
				PMM.set_bitmap_ptr(entry.start_address() as usize);
				polyfill::memset(PMM.get_bitmap_ptr(), 0xFF, bitmap_size);
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
			kiprintln!("Omitting bitmap from free address space...");
			size -= bitmap_size as u64;
			addr += bitmap_size as u64;
		}

		for bit in (0..size).step_by(PAGE_SIZE) {
			PMM.bitmap_reset_bit((addr + bit) as usize / PAGE_SIZE);
		}
	}

	kiprintln!("Initialized PMM bitmap at: {:#p}", PMM.get_bitmap_ptr());
}

unsafe impl GlobalAlloc for Pmm {
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		let pages = polyfill::div_up(layout.size(), PAGE_SIZE);
		let mut contiguous = 0;

		for offset in self.get_last_used_page()..self.get_highest_bit() {
			if !self.bitmap_test_bit(offset) {
				contiguous += 1;

				if contiguous == pages {
					let page = offset + 1 - contiguous;
					self.set_last_used_page(page);

					for p in page..page + contiguous {
						self.bitmap_set_bit(p);
					}

					return (page * PAGE_SIZE) as *mut u8;
				}
			} else {
				contiguous = 0;
			}
		}

		// oom i think? until paging is set up i guess
		panic!("PMM: OOM!");
	}

	unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
		let pages = polyfill::div_up(layout.size(), PAGE_SIZE);
		for page in 0..pages {
			self.bitmap_reset_bit((ptr as usize + page) / PAGE_SIZE)
		}
	}
}

pub fn sanity_check() {
	assert!(
		PMM.bitmap_test_bit(
			(PMM.get_bitmap_ptr() as usize) / PAGE_SIZE as usize
		),
		"Address space with bitmap marked as free: {}",
		unsafe { *PMM.get_bitmap_ptr() }
	);

	let ptr_to_int = unsafe { PMM.alloc(Layout::new::<u8>()) };
	unsafe {
		*ptr_to_int = 1u8;
	}

	assert!(
		PMM.bitmap_test_bit(ptr_to_int as usize / PAGE_SIZE),
		"Allocator failed to allocate u8! Allocated at: {:#p}",
		ptr_to_int
	);

	unsafe {
		PMM.dealloc(ptr_to_int, Layout::new::<u8>());
	};

	assert!(
		!PMM.bitmap_test_bit(ptr_to_int as usize / PAGE_SIZE),
		"Allocator failed to deallocate u8! Exists at: {:#p}",
		ptr_to_int
	);

	ksprintln!("PMM alloc/dealloc sanity checks passed!");
}
