use crate::{polyfill, STIVALE_STRUCT};
use core::fmt::{self, Write};
use lazy_static::lazy_static;
use spin::Mutex;
use stivale::framebuffer::FramebufferTag;

#[cfg(FONT = "LINUX")]
use linux_console_font::{FONT, FONT_DIMENSIONS};
#[cfg(FONT = "ZAP")]
use zap_font::{FONT, FONT_DIMENSIONS};

pub enum CommonColors {
	Red,
	Green,
	Cyan,
	White,
	Black,
}

impl Into<Pixel> for CommonColors {
	fn into(self) -> Pixel {
		match self {
			Self::Red => Pixel::new(255, 0, 0),
			Self::Green => Pixel::new(0, 255, 0),
			Self::Cyan => Pixel::new(0, 255, 255),
			Self::White => Pixel::new(255, 255, 255),
			Self::Black => Pixel::new(0, 0, 0),
		}
	}
}

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
	height: u16,
	size: usize,
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
			height: tag.height(),
			bpp: tag.bpp(),
			size: tag.size(),
			row: 0,
			col: 0,
			fg: Default::default(),
			bg: Default::default(),
		}
	}

	pub fn draw(&mut self, c: char) {
		match c {
			'\n' => {
				self.row += FONT_DIMENSIONS.1 as u16;
				self.col = 0;
				return;
			}
			'\t' => {
				for _ in 0..12 {
					self.draw(' ');
				}
				return;
			}
			_ => {
				let offset = (c as u8 - 32) as usize * 16;
				for y in 0..16 {
					for x in 0..8 {
						let cur_x = self.col as usize + (8 - x);
						let cur_y = self.row as usize + y;

						let ptr = (self.ptr
							+ (cur_x * (self.bpp / 8) as usize
								+ cur_y * self.pitch as usize) as usize)
							as *mut u32;

						if FONT[y + offset as usize] >> x & 1 == 1 {
							unsafe { *ptr = self.fg.as_bits() }
						} else {
							unsafe { *ptr = self.bg.as_bits() }
						}
					}
				}

				if self.row == self.height {
					for y in 0..FONT_DIMENSIONS.1 {
						for x in 0..self.pitch {
							let ptr = (self.ptr
								+ y as usize * self.pitch as usize
								+ x as usize) as *mut u32;
							unsafe { *ptr = 0 }
						}
					}

					unsafe {
						polyfill::memmove(
							self.ptr as *mut u8,
							(self.ptr
								+ self.pitch as usize
									* FONT_DIMENSIONS.1 as usize) as *mut u8,
							self.size,
						);
					}

					self.row -= 1;
				}
				if self.pitch == self.col {
					self.draw('\n');
				} else {
					self.col += FONT_DIMENSIONS.0 as u16;
				}
			}
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
		Mutex::new(FramebufferWriter::new(
			STIVALE_STRUCT
				.inner()
				.framebuffer()
				.expect("Framebuffer tag is empty!")
		));
}

/// Render formatted text to the framebuffer
#[macro_export]
macro_rules! kprint {
	($($arg:tt)+) => ({
		use core::fmt::Write;
		use crate::stdio::framebuffer::STDIO_WRITER;

		let _ = STDIO_WRITER.lock().write_fmt(format_args!($($arg)+));
	});
}

/// Render formatted text to the framebuffer, with a newline
#[macro_export]
macro_rules! kprintln {
	() => ({
		$crate::kprint!("\n");
	});
	($($arg:tt)+) => ({
		$crate::kprint!($($arg)+);
		$crate::kprint!("\n");
	});
}

/// Render formatted informative text to the framebuffer, with a newline & a
/// colored "info" label
#[macro_export]
macro_rules! kiprintln {
	($($arg:tt)+) => ({
		$crate::STDIO_WRITER.lock().fg.set($crate::CommonColors::Cyan);
		$crate::kprint!("[ info ] => ");
		$crate::STDIO_WRITER.lock().fg.reset();

		$crate::kprint!($($arg)+);
		$crate::kprintln!();
	});
}

/// Render formatted error text to the framebuffer, with a newline & a colored
/// "fail" label
#[macro_export]
macro_rules! keprintln {
	($($arg:tt)+) => ({
		$crate::STDIO_WRITER.lock().fg.set($crate::CommonColors::Red);
		$crate::kprint!("[ fail ] => ");
		$crate::STDIO_WRITER.lock().fg.reset();

		$crate::kprint!($($arg)+);
		$crate::kprintln!();
	});
}

/// Render formatted success text to the framebuffer, with a newline & a colored
/// "scss" label
#[macro_export]
macro_rules! ksprintln {
	($($arg:tt)+) => ({
		$crate::STDIO_WRITER.lock().fg.set($crate::CommonColors::Green);
		$crate::kprint!("[ scss ] => ");
		$crate::STDIO_WRITER.lock().fg.reset();

		$crate::kprint!($($arg)+);
		$crate::kprintln!();
	});
}
