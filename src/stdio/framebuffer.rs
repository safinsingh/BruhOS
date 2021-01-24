use super::{BPP, HEIGHT, WIDTH};
use crate::STIVALE_STRUCT;
use core::{
	cell::UnsafeCell,
	fmt::{self, Write},
};
use spin::Mutex;

#[cfg(FONT = "LINUX")]
use linux_console_font::{FONT, FONT_DIMENSIONS};
#[cfg(FONT = "ZAP")]
use zap_font::{FONT, FONT_DIMENSIONS};

pub struct FramebufferWriter {
	ptr: Option<usize>,
	pitch: Option<u16>,
	height: Option<u16>,
	size: Option<usize>,
	bpp: Option<u16>,
	row: u16,
	col: u16,
	pub fg: Option<Pixel>,
	pub bg: Option<Pixel>,
	buf: [u8; (WIDTH as usize * HEIGHT as usize * BPP as usize / 8)],
}

impl FramebufferWriter {
	pub const fn new() -> Self {
		Self {
			ptr: None,
			pitch: None,
			height: None,
			bpp: None,
			size: None,
			row: 0,
			col: 0,
			fg: None,
			bg: None,
			buf: [0; (WIDTH as usize * HEIGHT as usize * BPP as usize / 8)],
		}
	}

	pub fn init(&mut self) {
		let fb = STIVALE_STRUCT
			.inner()
			.framebuffer()
			.expect("Framebuffer tag is empty!");

		self.ptr = Some(fb.start_address());
		self.pitch = Some(fb.pitch());
		self.height = Some(fb.height());
		self.bpp = Some(fb.bpp());
		self.size = Some(fb.size());
		self.fg = Some(Default::default());
		self.bg = Some(Default::default());
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

						let base_offset =
							(cur_x * (self.bpp.unwrap() / 8) as usize
								+ cur_y * self.pitch.unwrap() as usize) as usize;

						if FONT[y + offset as usize] >> x & 1 == 1 {
							for byte in 0..4 {
								self.buf[base_offset + byte] =
									(self.fg.unwrap().as_bits() >> byte * 8)
										as u8 & 0xFF
							}
						} else {
							for byte in 0..4 {
								self.buf[base_offset + byte] =
									(self.bg.unwrap().as_bits() >> byte * 8)
										as u8 & 0xFF
							}
						}
					}
				}

				if self.pitch.unwrap() == self.col {
					self.draw('\n');
					return;
				} else {
					self.col += FONT_DIMENSIONS.0 as u16;
				}

				if self.row == self.height.unwrap() {
					let top_row_bytes = self.pitch.unwrap() as usize
						* FONT_DIMENSIONS.1 as usize;

					for offset in 0..top_row_bytes {
						unsafe {
							*(self.ptr.unwrap() as *mut u8).add(offset) = 0
						}
					}

					unsafe {
						core::ptr::copy(
							self.ptr.unwrap() as *mut u8,
							(self.ptr.unwrap() + top_row_bytes) as *mut u8,
							self.size.unwrap() - top_row_bytes,
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
				self.ptr.unwrap() as *mut u8,
				self.size.unwrap(),
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

pub struct FramebufferWriterWrapper(UnsafeCell<Mutex<FramebufferWriter>>);
unsafe impl Send for FramebufferWriterWrapper {}
unsafe impl Sync for FramebufferWriterWrapper {}

impl FramebufferWriterWrapper {
	pub fn inner(&self) -> &mut FramebufferWriter {
		unsafe { self.0.get().as_mut().unwrap().get_mut() }
	}
}

pub static STDIO_WRITER: FramebufferWriterWrapper = FramebufferWriterWrapper(
	UnsafeCell::new(Mutex::new(FramebufferWriter::new())),
);

/// Render formatted text to the framebuffer
#[macro_export]
macro_rules! kprint {
	($($arg:tt)+) => ({
		use core::fmt::Write;
		use $crate::stdio::framebuffer::STDIO_WRITER;

		let writer = STDIO_WRITER.inner();
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
		use $crate::stdio::framebuffer::STDIO_WRITER;
		let writer = STDIO_WRITER.inner();

		writer.fg.unwrap().set($crate::CommonColors::Cyan);
		$crate::kprint!("[ info ] => ");
		writer.fg.unwrap().set($crate::CommonColors::White);

		$crate::kprint!($($arg)+);
		$crate::kprintln!();
	});
}

/// Render formatted error text to the framebuffer, with a newline & a colored
/// "fail" label
#[macro_export]
macro_rules! keprintln {
	($($arg:tt)+) => ({
		use $crate::stdio::framebuffer::STDIO_WRITER;
		let writer = STDIO_WRITER.inner();

		writer.fg.unwrap().set($crate::CommonColors::Red);
		$crate::kprint!("[ fail ] => ");
		writer.fg.unwrap().set($crate::CommonColors::White);

		$crate::kprint!($($arg)+);
		$crate::kprintln!();
	});
}

/// Render formatted success text to the framebuffer, with a newline & a colored
/// "scss" label
#[macro_export]
macro_rules! ksprintln {
	($($arg:tt)+) => ({
		use $crate::stdio::framebuffer::STDIO_WRITER;
		let writer = STDIO_WRITER.inner();

		writer.fg.unwrap().set($crate::CommonColors::Green);
		$crate::kprint!("[ scss ] => ");
		writer.fg.unwrap().set($crate::CommonColors::White);

		$crate::kprint!($($arg)+);
		$crate::kprintln!();
	});
}
