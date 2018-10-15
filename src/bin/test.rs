extern crate trezor_api;

fn main() {
	let trezors = trezor_api::find_devices();
	println!("{:?}", trezors);
}
