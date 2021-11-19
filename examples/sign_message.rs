extern crate bitcoin;
extern crate fern;
extern crate hex;
extern crate log;
extern crate trezor;

use std::io;

use bitcoin::{network::constants::Network, util::bip32, Address};

use trezor::{InputScriptType, TrezorMessage, TrezorResponse};

fn setup_logger() {
	fern::Dispatch::new()
		.format(|out, message, record| {
			out.finish(format_args!("[{}][{}] {}", record.target(), record.level(), message))
		})
		.level(log::LevelFilter::Trace)
		.chain(std::io::stderr())
		.apply()
		.unwrap();
}

fn handle_interaction<T, R: TrezorMessage>(resp: TrezorResponse<T, R>) -> T {
	match resp {
		TrezorResponse::Ok(res) => res,
		TrezorResponse::Failure(_) => resp.ok().unwrap(), // assering ok() returns the failure error
		TrezorResponse::ButtonRequest(req) => handle_interaction(req.ack().unwrap()),
		TrezorResponse::PinMatrixRequest(req) => {
			println!("Enter PIN");
			let mut pin = String::new();
			if io::stdin().read_line(&mut pin).unwrap() != 5 {
				println!("must enter pin, received: {}", pin);
			}
			// trim newline
			handle_interaction(req.ack_pin(pin[..4].to_owned()).unwrap())
		}
		TrezorResponse::PassphraseRequest(req) => {
			println!("Enter passphrase");
			let mut pass = String::new();
			io::stdin().read_line(&mut pass).unwrap();
			// trim newline
			handle_interaction(req.ack_passphrase(pass[..pass.len() - 1].to_owned()).unwrap())
		}
		TrezorResponse::PassphraseStateRequest(req) => handle_interaction(req.ack().unwrap()),
	}
}

fn main() {
	setup_logger();
	// init with debugging
	let mut trezor = trezor::unique(true).unwrap();
	trezor.init_device().unwrap();

	let pubkey = handle_interaction(
		trezor
			.get_public_key(
				&vec![
					bip32::ChildNumber::from_hardened_idx(0).unwrap(),
					bip32::ChildNumber::from_hardened_idx(0).unwrap(),
					bip32::ChildNumber::from_hardened_idx(1).unwrap(),
				].into(),
				trezor::protos::InputScriptType::SPENDADDRESS,
				Network::Testnet,
				true,
			)
			.unwrap(),
	);
	let addr = Address::p2pkh(&pubkey.public_key, Network::Testnet);
	println!("address: {}", addr);

	let (addr, signature) = handle_interaction(
		trezor
			.sign_message(
				"regel het".to_owned(),
				&vec![
					bip32::ChildNumber::from_hardened_idx(0).unwrap(),
					bip32::ChildNumber::from_hardened_idx(0).unwrap(),
					bip32::ChildNumber::from_hardened_idx(1).unwrap(),
				].into(),
				InputScriptType::SPENDADDRESS,
				Network::Testnet,
			)
			.unwrap(),
	);
	println!("Addr from device: {}", addr);
	println!("Signature: {:?}", signature);
}
