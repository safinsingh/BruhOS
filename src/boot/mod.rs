use core::cell::UnsafeCell;
use stivale::{HeaderFramebufferTag, StivaleHeader, StivaleStructure};

#[link_section = ".stivale2hdr"]
#[used]
pub static STIVALE_HDR: StivaleHeader =
	StivaleHeader::new(STACK[0] as *const u8)
		.tags((&FRAMEBUFFER_TAG as *const HeaderFramebufferTag).cast());

static STACK: [u8; 4096] = [0; 4096];
static FRAMEBUFFER_TAG: HeaderFramebufferTag =
	HeaderFramebufferTag::new().bpp(32);

pub struct StivaleInfo(UnsafeCell<Option<StivaleStructure>>);
unsafe impl Send for StivaleInfo {}
unsafe impl Sync for StivaleInfo {}

impl StivaleInfo {
	pub unsafe fn set(&self, to: StivaleStructure) {
		*self.0.get() = Some(to)
	}

	pub fn inner(&self) -> &StivaleStructure {
		// SAFETY: safe assuming it's called after STIVALE_STRUCT is set
		// properly
		unsafe {
			self.0
				.get()
				.as_ref()
				.expect("Stivale struct was not yet initialized!")
				.as_ref()
				.expect("Stivale struct is empty!")
		}
	}
}

pub static STIVALE_STRUCT: StivaleInfo = StivaleInfo(UnsafeCell::new(None));
