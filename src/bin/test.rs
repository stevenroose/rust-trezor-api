extern crate trezor_api;

use std::io;

use trezor_api::TrezorClient;

fn main() {
	let mut trezor = trezor_api::unique(Some(true)).unwrap();
	trezor.init_device().unwrap();
	trezor.change_pin(false).unwrap();
	let _: trezor_api::protos::PinMatrixRequest = trezor.button_ack().unwrap();
	let mut pin = String::new();
	if io::stdin().read_line(&mut pin).unwrap() != 5 {
		println!("must enter pin, received: {}", pin);
	}
	println!("{:?}", trezor.pin_matrix_ack(pin[..4].to_owned()).unwrap())
}
