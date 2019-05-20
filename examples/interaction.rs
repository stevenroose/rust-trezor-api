extern crate bitcoin;
extern crate trezor;

use std::io;

use bitcoin::{network::constants::Network, util::bip32, Address};
use trezor::{Error, TrezorMessage, TrezorResponse};

fn handle_interaction<T, R: TrezorMessage>(resp: TrezorResponse<T, R>) -> Result<T, Error> {
	match resp {
		TrezorResponse::Ok(res) => Ok(res),
		TrezorResponse::Failure(_) => resp.ok(), // assering ok() returns the failure error
		TrezorResponse::ButtonRequest(req) => handle_interaction(req.ack()?),
		TrezorResponse::PinMatrixRequest(req) => {
			println!("Enter PIN");
			let mut pin = String::new();
			if io::stdin().read_line(&mut pin).unwrap() != 5 {
				println!("must enter pin, received: {}", pin);
			}
			// trim newline
			handle_interaction(req.ack_pin(pin[..4].to_owned())?)
		}
		TrezorResponse::PassphraseRequest(req) => {
			println!("Enter passphrase");
			let mut pass = String::new();
			io::stdin().read_line(&mut pass).unwrap();
			// trim newline
			handle_interaction(req.ack_passphrase(pass[..pass.len() - 1].to_owned())?)
		}
		TrezorResponse::PassphraseStateRequest(req) => handle_interaction(req.ack()?),
	}
}

fn do_main() -> Result<(), trezor::Error> {
	// init with debugging
	let mut trezor = trezor::unique(true)?;
	trezor.init_device()?;

	let xpub = handle_interaction(trezor.get_public_key(
		vec![
			bip32::ChildNumber::from_hardened_idx(0).unwrap(),
			bip32::ChildNumber::from_hardened_idx(0).unwrap(),
			bip32::ChildNumber::from_hardened_idx(0).unwrap(),
		],
		trezor::protos::InputScriptType::SPENDADDRESS,
		Network::Testnet,
		true,
	)?)?;
	println!("{}", xpub);
	println!("{:?}", xpub);
	println!("{}", Address::p2pkh(&xpub.public_key, Network::Testnet));

	Ok(())
}

fn main() {
	do_main().unwrap()
}
