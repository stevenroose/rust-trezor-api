extern crate bitcoin;
extern crate hid;
extern crate protobuf;
extern crate secp256k1;

use std::fmt;

mod constants;
mod error;
mod protocol;
mod protos;
mod trezor;

pub use error::*;
pub use trezor::*;

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Model {
	Trezor1,
	Trezor2,
	Trezor2Bl,
}

impl fmt::Display for Model {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.write_str(match self {
			Model::Trezor1 => "Trezor 1",
			Model::Trezor2 => "Trezor 2",
			Model::Trezor2Bl => "Trezor 2 Bl",
		})
	}
}
