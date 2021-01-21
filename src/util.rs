#[inline(always)]
pub fn div_up(a: usize, b: usize) -> usize {
	let r = a / b;
	if a % b != 0 {
		r + 1
	} else {
		r
	}
}
