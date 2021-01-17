static CONFIG: &[(&str, &str)] = &[
	// Can be either "LINUX" or "ZAP"
	("FONT", "ZAP"),
];

fn main() {
	for (key, value) in CONFIG.iter() {
		println!("cargo:rustc-cfg={}=\"{}\"", key, value);
	}
}
