extern crate bitcoin;
extern crate byteorder;
extern crate hid;
extern crate protobuf;
extern crate secp256k1;

mod client;
mod constants;
mod error;
mod messages;
pub mod protos;
mod transport;

pub use client::{
	ButtonRequest, InteractionType, PassphraseRequest, PinMatrixRequest, Trezor, TrezorResponse,
};
pub use error::{Error, Result};

use std::fmt;

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
			Model::Trezor2Bl => "Trezor 2 Bootloader",
		})
	}
}

#[derive(Debug)]
pub struct AvailableDevice {
	pub model: Model,
	pub debug: bool,
	transport: transport::AvailableDeviceTransport,
}

impl fmt::Display for AvailableDevice {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{} (transport: {}) (debug: {})", self.model, &self.transport, self.debug)
	}
}

impl AvailableDevice {
	pub fn connect(self) -> Result<Trezor> {
		let transport = transport::connect(&self)?;
		Ok(Trezor::new(self.model, transport))
	}
}

/// Search for all available devices.
pub fn find_devices() -> Result<Vec<AvailableDevice>> {
	transport::hid::HidTransport::find_devices()
}

/// Try to get a single device.  Optionally specify whether debug should be enabled or not.
/// Can error if there are multiple or no devices available.
/// For more fine-grained device selection, use `find_devices()`.
pub fn unique(debug: Option<bool>) -> Result<Trezor> {
	let mut devices = find_devices()?;
	if let Some(debug) = debug {
		devices.retain(|d| d.debug == debug);
	}
	match devices.len() {
		0 => Err(Error::NoDeviceFound),
		1 => Ok(devices.remove(0).connect()?),
		_ => Err(Error::DeviceNotUnique),
	}
}
