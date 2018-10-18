extern crate trezor_api;

use trezor_api::TrezorClient;

fn main() {
	let mut trezor = trezor_api::unique(Some(true)).unwrap();
	trezor.change_pin(false).unwrap();
}
