extern crate bitcoin;
extern crate byteorder;
extern crate hex;
extern crate hid;
#[macro_use]
extern crate log;
extern crate protobuf;
extern crate secp256k1;

mod client;
mod constants;
mod error;
mod messages;
mod transport;

// Public to allow custom use of the `Trezor::call` method for unsupported currencies etc.
pub mod protos;
pub use client::*;
pub use error::{Error, Result};
pub use messages::TrezorMessage;

use std::fmt;

///! # Trezor API library
///!
///!
///! ## Logging
///! We use the log package interface, so any logger that supports log can be attached.
///! Please be aware that `trace` logging can contain sensitive data.

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
		_ => {
			debug!("Trezor devices found: {:?}", devices);
			Err(Error::DeviceNotUnique)
		}
	}
}
