use core::fmt::{self, Write};
use linux_console_font::FONT;
use stivale::framebuffer::FramebufferTag;

pub struct FramebufferWriter {
	ptr: usize,
	pitch: u16,
	bpp: u16,
	row: u16,
	col: u16,
}

impl FramebufferWriter {
	pub fn new(tag: &FramebufferTag) -> Self {
		Self {
			ptr: tag.start_address(),
			pitch: tag.pitch(),
			bpp: tag.bpp(),
			row: 0,
			col: 0,
		}
	}

	pub fn draw(&mut self, c: char) {
		if c as u8 == b'\n' {
			self.row += (self.bpp / 8) * 2;
			self.col = 0;
			return;
		}

		let offset = (c as u8 - 32) as usize * 16;
		for y in 0..16 {
			for x in 0..8 {
				let cur_x = self.col + (8 - x);
				let cur_y = self.row + y;

				if FONT[(y + offset as u16) as usize] >> x & 1 == 1 {
					// TODO: colors and stuff
					unsafe {
						*((self.ptr
							+ (cur_x * (self.bpp / 8) + cur_y * self.pitch)
								as usize) as *mut u32) = 0xFF0000
					}
				}
			}
		}

		if self.pitch == self.col {
			self.draw('\n');
		} else {
			self.col += (self.bpp / 8) * 2;
		}
	}
}

impl Write for FramebufferWriter {
	fn write_str(&mut self, s: &str) -> fmt::Result {
		Ok(for c in s.chars() {
			match c {
				c if c.is_ascii() => self.draw(c),
				_ => return Err(fmt::Error),
			}
		})
	}
}

#[macro_export]
macro_rules! kprint {
	($writer:ident, $($arg:tt)+) => {
		use core::fmt::Write;
		let _ = $writer.write_str(concat!($($arg)+));
	};
}
