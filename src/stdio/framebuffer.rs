use crate::STIVALE_STRUCT;
use core::fmt::{self, Write};
use lazy_static::lazy_static;
use spin::Mutex;
use stivale::framebuffer::FramebufferTag;
use zap_font::FONT;

pub struct Pixel {
	r: u8,
	g: u8,
	b: u8,
}

impl Pixel {
	fn as_bits(&self) -> u32 {
		(self.r as u32) << 16 | (self.g as u32) << 8 | self.b as u32
	}

	pub fn set_cyan(&mut self) {
		self.r = 0;
		self.g = 255;
		self.b = 255;
	}

	pub fn reset(&mut self) {
		self.r = 255;
		self.g = 255;
		self.b = 255;
	}
}

impl Default for Pixel {
	fn default() -> Self {
		Self { r: 0, g: 0, b: 0 }
	}
}

pub struct FramebufferWriter {
	ptr: usize,
	pitch: u16,
	bpp: u16,
	row: u16,
	col: u16,
	pub fg: Pixel,
	pub bg: Pixel,
}

impl FramebufferWriter {
	pub fn new(tag: &FramebufferTag) -> Self {
		Self {
			ptr: tag.start_address(),
			pitch: tag.pitch(),
			bpp: tag.bpp(),
			row: 0,
			col: 0,
			fg: Default::default(),
			bg: Default::default(),
		}
	}

	pub fn draw(&mut self, c: char) {
		if c as u8 == b'\n' {
			self.row += 16;
			self.col = 0;
			return;
		}

		let offset = (c as u8 - 32) as usize * 16;
		for y in 0..16 {
			for x in 0..8 {
				let cur_x = self.col + (8 - x);
				let cur_y = self.row + y;

				let ptr = (self.ptr
					+ (cur_x * (self.bpp / 8) + cur_y * self.pitch) as usize)
					as *mut u32;

				if FONT[(y + offset as u16) as usize] >> x & 1 == 1 {
					unsafe { *ptr = self.fg.as_bits() }
				} else {
					unsafe { *ptr = self.bg.as_bits() }
				}
			}
		}

		if self.pitch == self.col {
			self.draw('\n');
		} else {
			self.col += 8;
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

lazy_static! {
	pub static ref STDIO_WRITER: Mutex<FramebufferWriter> =
		Mutex::new(FramebufferWriter::new(unsafe {
			STIVALE_STRUCT
				.0
				.get()
				.as_ref()
				.unwrap()
				.as_ref()
				.unwrap()
				.framebuffer()
				.unwrap()
		}));
}

#[macro_export]
macro_rules! kprint {
	($($arg:tt)+) => ({
		use core::fmt::Write;
		use crate::stdio::framebuffer::STDIO_WRITER;
		let _ = STDIO_WRITER.lock().write_fmt(format_args!($($arg)+));
	});
}

#[macro_export]
macro_rules! kprintln {
	() => ({
		kprint!("\n");
	});
	($($arg:tt)+) => ({
		kprint!($($arg)+);
		kprintln!();
	});
}

#[macro_export]
macro_rules! kiprintln {
	($($arg:tt)+) => ({
		use crate::stdio::framebuffer::STDIO_WRITER;

		STDIO_WRITER.lock().fg.set_cyan();
		kprint!("[ info ] => ");
		STDIO_WRITER.lock().fg.reset();

		kprint!($($arg)+);
		kprintln!();
	});
}
