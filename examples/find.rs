extern crate trezor;

fn main() {
	let trezors = trezor::find_devices(false).unwrap();
	println!("Found {} devices: ", trezors.len());
	for t in trezors.into_iter() {
		println!("- {}", t);
		{
			let mut client = t.connect().unwrap();
			println!("{:?}", client.initialize().unwrap());
		}
	}
}
