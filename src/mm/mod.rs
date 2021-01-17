pub const HIGH_HALF_OFFSET: usize = 0xffff800000000000;
pub const PAGE_SIZE: usize = 4096;

pub fn div_up(a: usize, b: usize) -> usize {
	// https://www.reddit.com/r/rust/comments/bk7v15/my_next_favourite_way_to_divide_integers_rounding/
	(0..a).step_by(b).size_hint().0
}

pub mod pmm;
