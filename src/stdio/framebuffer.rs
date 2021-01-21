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

pub struct DoubleBufferInner {
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

pub struct DoubleBuffer(pub UnsafeCell<DoubleBufferInner>);

impl DoubleBuffer {
	pub fn new(tag: &FramebufferTag) -> Self {
		Self(UnsafeCell::new(DoubleBufferInner {
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
		}))
	}

	pub fn draw(&mut self, c: char) {
		let mut inner = self.0.get_mut();

		match c {
			'\r' => {
				inner.col = 0;
			}
			'\n' => {
				inner.row += FONT_DIMENSIONS.1 as u16;
			}
			'\t' => {
				drop(inner);
				for _ in 0..12 {
					self.draw(' ');
				}
			}
			_ => {
				let offset = (c as u8 - 32) as usize * 16;
				for y in 0..16 {
					for x in 0..8 {
						let cur_x = inner.col as usize + (8 - x);
						let cur_y = inner.row as usize + y;

						let base_offset = (cur_x * (inner.bpp / 8) as usize
							+ cur_y * inner.pitch as usize)
							as usize;

						if FONT[y + offset as usize] >> x & 1 == 1 {
							for byte in 0..4 {
								inner.buf[base_offset + byte] =
									(inner.fg.as_bits() >> byte * 8) as u8
										& 0xFF
							}
						} else {
							for byte in 0..4 {
								inner.buf[base_offset + byte] =
									(inner.bg.as_bits() >> byte * 8) as u8
										& 0xFF
							}
						}
					}
				}

				if inner.pitch == inner.col {
					drop(inner);
					self.draw('\n');
					return;
				} else {
					inner.col += FONT_DIMENSIONS.0 as u16;
				}

				if inner.row == inner.height {
					let top_row_bytes =
						inner.pitch as usize * FONT_DIMENSIONS.1 as usize;

					for offset in 0..top_row_bytes {
						unsafe { *(inner.ptr as *mut u8).add(offset) = 0 }
					}

					unsafe {
						core::ptr::copy(
							inner.ptr as *mut u8,
							(inner.ptr + top_row_bytes) as *mut u8,
							inner.size - top_row_bytes,
						);
					}

					inner.row -= 1;
				}
			}
		}
	}

	pub fn flush(&self) {
		let inner = unsafe { self.0.get().as_ref() }.unwrap();

		unsafe {
			core::ptr::copy(
				&inner.buf as *const _ as *mut _,
				inner.ptr as *mut u8,
				inner.size,
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

	pub fn reset(&mut self) {
		self.r = 255;
		self.g = 255;
		self.b = 255;
	}
}

pub struct FramebufferWriter(pub DoubleBuffer);

impl FramebufferWriter {
	pub fn new() -> Self {
		let tag = STIVALE_STRUCT
			.inner()
			.framebuffer()
			.expect("Framebuffer tag is empty!");

		Self(DoubleBuffer::new(tag))
	}

	pub fn flush(&self) {
		self.0.flush()
	}
}

impl Write for FramebufferWriter {
	fn write_str(&mut self, s: &str) -> fmt::Result {
		for c in s.chars() {
			match c {
				c if c.is_ascii() => self.0.draw(c),
				_ => return Err(fmt::Error),
			}
		}
		Ok(())
	}
}

lazy_static! {
	pub static ref STDIO_WRITER: Mutex<FramebufferWriter> =
		Mutex::new(FramebufferWriter::new());
}

/// Render formatted text to the framebuffer
#[macro_export]
macro_rules! kprint {
	($($arg:tt)+) => ({
		use core::fmt::Write;
		use crate::stdio::framebuffer::{STDIO_WRITER, FramebufferWriter};

		let writer = unsafe {
			(&STDIO_WRITER.lock() as *const _ as *mut FramebufferWriter).as_mut().unwrap()
		};

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
		let writer = unsafe {
			$crate::STDIO_WRITER.lock()
				.0
				.0
				.get()
				.as_mut()
				.unwrap()
		};

		writer.fg.set($crate::CommonColors::Cyan);
		$crate::kprint!("[ info ] => ");
		writer.fg.reset();

		$crate::kprint!($($arg)+);
		$crate::kprintln!();
	});
}

/// Render formatted error text to the framebuffer, with a newline & a colored
/// "fail" label
#[macro_export]
macro_rules! keprintln {
	($($arg:tt)+) => ({
		let writer = unsafe {
			$crate::STDIO_WRITER.lock()
				.0
				.0
				.get()
				.as_mut()
				.unwrap()
		};

		writer.fg.set($crate::CommonColors::Red);
		$crate::kprint!("[ fail ] => ");
		writer.fg.reset();

		$crate::kprint!($($arg)+);
		$crate::kprintln!();
	});
}

/// Render formatted success text to the framebuffer, with a newline & a colored
/// "scss" label
#[macro_export]
macro_rules! ksprintln {
	($($arg:tt)+) => ({
		let writer = unsafe {
			$crate::STDIO_WRITER.lock()
				.0
				.0
				.get()
				.as_mut()
				.unwrap()
		};

		writer.fg.set($crate::CommonColors::Green);
		$crate::kprint!("[ scss ] => ");
		writer.fg.reset();

		$crate::kprint!($($arg)+);
		$crate::kprintln!();
	});
}
