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

	#[inline(always)]
	unsafe fn set_bitmap_ptr(&self, to: usize) {
		self.0.get().as_ref().unwrap().lock().bitmap_ptr = Some(to);
	}

	#[inline(always)]
	unsafe fn set_highest_bit(&self, to: usize) {
		self.0.get().as_ref().unwrap().lock().highest_bit = Some(to);
	}

	#[inline(always)]
	unsafe fn set_last_used_page(&self, to: usize) {
		self.0.get().as_ref().unwrap().lock().last_used_page = to;
	}

	#[inline(always)]
	fn get_bitmap_ptr(&self) -> *mut u8 {
		unsafe {
			self.0.get().as_ref().unwrap().lock().bitmap_ptr.unwrap() as *mut u8
		}
	}

	#[inline(always)]
	fn get_highest_bit(&self) -> usize {
		unsafe { self.0.get().as_ref().unwrap().lock().highest_bit.unwrap() }
	}

	#[inline(always)]
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

	let highest_page = polyfill::div_up(
		mmap_usable
			.clone()
			.fold(0, |acc, cur| cur.end_address().max(acc)) as usize,
		super::PAGE_SIZE,
	);

	// highest page / page size / 8 bits per byte
	unsafe {
		PMM.set_highest_bit(highest_page);
	}

	let bitmap_size = highest_page / 8;
	kiprintln!(
		"Addressing: {} MiB of memory",
		(highest_page * 4096) / 1024 / 1024
	);

	let mut bitmap_entry = 0;
	for (idx, entry) in mmap_usable.clone().enumerate() {
		if entry.size() >= bitmap_size as u64 {
			unsafe {
				PMM.set_bitmap_ptr(entry.start_address() as usize);
				polyfill::memset(PMM.get_bitmap_ptr(), 0xFF, bitmap_size);
			}

			bitmap_entry = idx;
			break;
		}
	}

	// consume because we don't need it anymore
	for (idx, entry) in mmap_usable.enumerate() {
		let mut size = entry.size();
		let mut addr = entry.start_address();

		if idx == bitmap_entry {
			size -= bitmap_size as u64;
			addr += bitmap_size as u64;
		}

		for bit in (0..size).step_by(PAGE_SIZE) {
			PMM.bitmap_reset_bit((addr + bit) as usize / PAGE_SIZE);
		}
	}

	kiprintln!(
		"Initialized {} KiB PMM bitmap at: {:p}",
		bitmap_size / 1024,
		PMM.get_bitmap_ptr()
	);
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
		panic!("PMM: um");
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
		"Allocator failed to allocate test u8 correctly! Allocated at: {:#p}",
		ptr_to_int
	);

	unsafe {
		PMM.dealloc(ptr_to_int, Layout::new::<u8>());
	};

	assert!(
		!PMM.bitmap_test_bit(ptr_to_int as usize / PAGE_SIZE),
		"Allocator failed to deallocate test u8! Still exists at: {:#p}",
		ptr_to_int
	);

	ksprintln!("PMM alloc/dealloc sanity checks passed!");
}
