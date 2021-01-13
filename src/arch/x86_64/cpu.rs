pub fn wait_for_interrupt() {
	unsafe { asm!("hlt", options(noreturn)) }
}
