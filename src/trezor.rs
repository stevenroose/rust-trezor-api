use std::fmt;

use hid;

use super::Model;
use constants;
use error::{Error, Result};
use protobuf;
use protocol::{Protocol, ProtocolV1, ProtocolV2, Transport};

pub struct Trezor {
	pub model: Model,
	pub debug: bool,

	_hid_manager: hid::Manager,
	hid_version: usize,
	handle: Option<hid::Handle>,
}

impl Drop for Trezor {
	fn drop(&mut self) {
		// Manually drop before manager is dropped.
		self.handle.take();
	}
}

impl fmt::Debug for Trezor {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{} (debug: {}, hid version: {})", self.model, self.debug, self.hid_version)
	}
}

impl fmt::Display for Trezor {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{} (debug: {})", self.model, self.debug)
	}
}

impl Transport for Trezor {
	fn write_chunk(&mut self, chunk: Vec<u8>) -> Result<()> {
		assert_eq!(64, chunk.len());
		Ok(())
	}
	fn read_chunk(&mut self) -> Result<Vec<u8>> {
		Ok(vec![])
	}
}

impl Trezor {
	fn call<S, R>(&mut self, send: S) -> Result<R>
	where
		S: protobuf::Message,
		R: protobuf::Message,
	{
		Err(Error::UnknownHidVersion)
	}
}

/// Used to indicate that a Trezor device was found, but that it was unavailable for usage.
#[derive(Debug)]
pub struct AvailableDevice {
	pub model: Model,
	pub debug: bool,
	pub serial_nb: String,
}

impl fmt::Display for AvailableDevice {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{} (debug: {}, serial nb: {})", self.model, self.debug, &self.serial_nb)
	}
}

/// Probe the HID version for a Trezor 1 device.
fn probe_hid_version(handle: &mut hid::Handle) -> Result<usize> {
	let mut w = vec![0xff; 65];
	w[0] = 0;
	w[1] = 63;
	if handle.data().write(w)? == 65 {
		return Ok(2);
	}
	let mut w = vec![0xff; 64];
	w[0] = 63;
	if handle.data().write(w)? == 64 {
		return Ok(1);
	}
	Err(Error::UnknownHidVersion)
}

impl AvailableDevice {
	/// Convert the available device in a connected Trezor client.
	pub fn connect(self) -> Result<Trezor> {
		// Traverse all devices again and find the matching one.
		let hidman = hid::init()?;

		let mut found = None;
		for dev in hidman.devices() {
			if derive_model(&dev) == Some(self.model.clone())
				&& derive_debug(&dev) == Some(self.debug)
				&& dev.serial_number() == Some(self.serial_nb.clone())
			{
				found = Some(dev.open()?);
				break;
			}
		}

		if let Some(mut handle) = found {
			let hid_version = probe_hid_version(&mut handle)?;
			Ok(Trezor {
				model: self.model,
				debug: self.debug,
				_hid_manager: hidman,
				hid_version: hid_version,
				handle: Some(handle),
			})
		} else {
			Err(Error::NoDeviceFound)
		}
	}
}

fn derive_model(dev: &hid::Device) -> Option<Model> {
	match (dev.vendor_id(), dev.product_id()) {
		constants::hid::DEV_TREZOR1 => Some(Model::Trezor1),
		constants::hid::DEV_TREZOR2 => Some(Model::Trezor2),
		constants::hid::DEV_TREZOR2_BL => Some(Model::Trezor2Bl),
		_ => None,
	}
}

/// Derive from the HID device whether or not it is a debugable device or not.
/// It returns None for not-recognized devices.
fn derive_debug(dev: &hid::Device) -> Option<bool> {
	if dev.usage_page() == constants::hid::DEBUGLINK_USAGE
		|| dev.interface_number() == constants::hid::DEBUGLINK_INTERFACE
	{
		Some(true)
	} else if dev.usage_page() == constants::hid::WIRELINK_USAGE
		|| dev.interface_number() == constants::hid::WIRELINK_INTERFACE
	{
		Some(false)
	} else {
		None
	}
}

/// Look for connected Trezor devices.
/// This method returns a tuple of the avilable devices and unavailable devices.
pub fn find_devices() -> Result<Vec<AvailableDevice>> {
	let hidman = hid::init()?;
	let mut found = Vec::new();
	for dev in hidman.devices() {
		let model = match derive_model(&dev) {
			Some(m) => m,
			None => continue,
		};
		let debug = match derive_debug(&dev) {
			Some(d) => d,
			None => continue,
		};
		let serial = match dev.serial_number() {
			Some(s) => s.clone(),
			None => continue,
		};

		found.push(AvailableDevice {
			model: model,
			debug: debug,
			serial_nb: serial,
		});
	}
	Ok(found)
}

/// Try to get a single device.  Optionally specify whether debug should be enabled or not.
/// Can error if there are multiple or no devices available.
/// For more fine-grained device selection, use `find_devices()` and `find_devices_with_hid()`.
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
