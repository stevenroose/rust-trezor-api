extern crate trezor_api;

fn main() {
	let trezors = trezor_api::find_devices().unwrap();
	println!("Found {} devices: ", trezors.len());
	for t in trezors.into_iter() {
		println!("- {}", t);
		{
			let mut client = t.connect().unwrap();
			println!("{:?}", client.initialize().unwrap());
		}
	}
}
