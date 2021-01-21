use super::{BPP, HEIGHT, WIDTH};
use crate::STIVALE_STRUCT;
use core::{
	cell::UnsafeCell,
	fmt::{self, Write},
};
use lazy_static::lazy_static;
use spin::Mutex;
use stivale::framebuffer::FramebufferTag;

#[cfg(FONT = "LINUX")]
use linux_console_font::{FONT, FONT_DIMENSIONS};
#[cfg(FONT = "ZAP")]
use zap_font::{FONT, FONT_DIMENSIONS};

pub struct FramebufferWriter {
	ptr: usize,
	pitch: u16,
	height: u16,
	size: usize,
	bpp: u16,
	row: u16,
	col: u16,
	pub fg: Pixel,
	pub bg: Pixel,
	buf: [u8; (WIDTH as usize * HEIGHT as usize * BPP as usize / 8)],
}

impl FramebufferWriter {
	pub fn new(tag: &FramebufferTag) -> Self {
		Self {
			ptr: tag.start_address(),
			pitch: tag.pitch(),
			height: tag.height(),
			bpp: tag.bpp(),
			size: tag.size(),
			row: 0,
			col: 0,
			fg: Default::default(),
			bg: Default::default(),
			buf: [0; (WIDTH as usize * HEIGHT as usize * BPP as usize / 8)],
		}
	}

	pub fn draw(&mut self, c: char) {
		match c {
			'\r' => {
				self.col = 0;
			}
			'\n' => {
				self.row += FONT_DIMENSIONS.1 as u16;
			}
			'\t' => {
				for _ in 0..12 {
					self.draw(' ');
				}
			}
			_ => {
				let offset = (c as u8 - 32) as usize * 16;
				for y in 0..16 {
					for x in 0..8 {
						let cur_x = self.col as usize + (8 - x);
						let cur_y = self.row as usize + y;

						let base_offset = (cur_x * (self.bpp / 8) as usize
							+ cur_y * self.pitch as usize)
							as usize;

						if FONT[y + offset as usize] >> x & 1 == 1 {
							for byte in 0..4 {
								self.buf[base_offset + byte] =
									(self.fg.as_bits() >> byte * 8) as u8 & 0xFF
							}
						} else {
							for byte in 0..4 {
								self.buf[base_offset + byte] =
									(self.bg.as_bits() >> byte * 8) as u8 & 0xFF
							}
						}
					}
				}

				if self.pitch == self.col {
					self.draw('\n');
					return;
				} else {
					self.col += FONT_DIMENSIONS.0 as u16;
				}

				if self.row == self.height {
					let top_row_bytes =
						self.pitch as usize * FONT_DIMENSIONS.1 as usize;

					for offset in 0..top_row_bytes {
						unsafe { *(self.ptr as *mut u8).add(offset) = 0 }
					}

					unsafe {
						core::ptr::copy(
							self.ptr as *mut u8,
							(self.ptr + top_row_bytes) as *mut u8,
							self.size - top_row_bytes,
						);
					}

					self.row -= 1;
				}
			}
		}
	}

	pub fn flush(&self) {
		unsafe {
			core::ptr::copy(
				&self.buf as *const _ as *mut _,
				self.ptr as *mut u8,
				self.size,
			);
		}
	}
}

pub enum CommonColors {
	Red,
	Green,
	Cyan,
	White,
	Black,
}

impl From<CommonColors> for Pixel {
	fn from(c: CommonColors) -> Self {
		match c {
			CommonColors::Red => Self::new(255, 0, 0),
			CommonColors::Green => Self::new(0, 255, 0),
			CommonColors::Cyan => Self::new(0, 255, 255),
			CommonColors::White => Self::new(255, 255, 255),
			CommonColors::Black => Self::new(0, 0, 0),
		}
	}
}

#[derive(Default, Copy, Clone)]
pub struct Pixel {
	r: u8,
	g: u8,
	b: u8,
}

impl Pixel {
	fn as_bits(&self) -> u32 {
		(self.r as u32) << 16 | (self.g as u32) << 8 | self.b as u32
	}

	pub fn new(r: u8, g: u8, b: u8) -> Self {
		Self { r, g, b }
	}

	pub fn set(&mut self, to: impl Into<Pixel>) {
		let to: Pixel = to.into();
		self.r = to.r;
		self.g = to.g;
		self.b = to.b;
	}
}

impl Write for FramebufferWriter {
	fn write_str(&mut self, s: &str) -> fmt::Result {
		for c in s.chars() {
			match c {
				c if c.is_ascii() => self.draw(c),
				_ => return Err(fmt::Error),
			}
		}
		Ok(())
	}
}

lazy_static! {
	pub static ref STDIO_WRITER: Mutex<UnsafeCell<FramebufferWriter>> =
		Mutex::new(UnsafeCell::new(FramebufferWriter::new(
			STIVALE_STRUCT
				.inner()
				.framebuffer()
				.expect("Framebuffer tag is empty!")
		)));
}

#[inline(always)]
pub fn get_stdio_writer_mut<'a>() -> &'a mut FramebufferWriter {
	unsafe { STDIO_WRITER.lock().get().as_mut().unwrap() }
}

/// Render formatted text to the framebuffer
#[macro_export]
macro_rules! kprint {
	($($arg:tt)+) => ({
		use core::fmt::Write;
		use $crate::stdio::framebuffer::get_stdio_writer_mut;

		let writer = get_stdio_writer_mut();
		let _ = writer.write_fmt(format_args!($($arg)+));
		writer.flush();
	});
}

/// Render formatted text to the framebuffer, with a newline
#[macro_export]
macro_rules! kprintln {
	() => ({
		$crate::kprint!("\r\n");
	});
	($($arg:tt)+) => ({
		$crate::kprint!($($arg)+);
		$crate::kprint!("\r\n");
	});
}

/// Render formatted informative text to the framebuffer, with a newline & a
/// colored "info" label
#[macro_export]
macro_rules! kiprintln {
	($($arg:tt)+) => ({
		use $crate::stdio::framebuffer::get_stdio_writer_mut;
		let writer = get_stdio_writer_mut();

		writer.fg.set($crate::CommonColors::Cyan);
		$crate::kprint!("[ info ] => ");
		writer.fg.set($crate::CommonColors::White);

		$crate::kprint!($($arg)+);
		$crate::kprintln!();
	});
}

/// Render formatted error text to the framebuffer, with a newline & a colored
/// "fail" label
#[macro_export]
macro_rules! keprintln {
	($($arg:tt)+) => ({
		use $crate::stdio::framebuffer::get_stdio_writer_mut;
		let writer = get_stdio_writer_mut();

		writer.fg.set($crate::CommonColors::Red);
		$crate::kprint!("[ fail ] => ");
		writer.fg.set($crate::CommonColors::White);

		$crate::kprint!($($arg)+);
		$crate::kprintln!();
	});
}

/// Render formatted success text to the framebuffer, with a newline & a colored
/// "scss" label
#[macro_export]
macro_rules! ksprintln {
	($($arg:tt)+) => ({
		use $crate::stdio::framebuffer::get_stdio_writer_mut;
		let writer = get_stdio_writer_mut();

		writer.fg.set($crate::CommonColors::Green);
		$crate::kprint!("[ scss ] => ");
		writer.fg.set($crate::CommonColors::White);

		$crate::kprint!($($arg)+);
		$crate::kprintln!();
	});
}
