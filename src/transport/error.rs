//! # Error Handling

use std::error;
use std::fmt;

use hid;
use libusb;

/// Trezor error.
#[derive(Debug)]
pub enum Error {
	/// Error from hidapi.
	Hid(hid::Error),
	/// Error from libusb.
	Usb(libusb::Error),
	/// The device to connect to was not found.
	DeviceNotFound,
	/// The device is no longer available.
	DeviceDisconnected,
	/// The HID version supported by the device was unknown.
	UnknownHidVersion,
	/// The device produced a data chunk of unexpected size.
	UnexpectedChunkSizeFromDevice(usize),
	/// Timeout expired while reading from device.
	DeviceReadTimeout,
	/// The device sent a chunk with a wrong magic value.
	DeviceBadMagic,
	/// The device sent a message with a wrong session id.
	DeviceBadSessionId,
	/// The device sent an unexpected sequence number.
	DeviceUnexpectedSequenceNumber,
	/// Received a non-existing message type from the device.
	InvalidMessageType(u32),
	/// Unable to determine device serial number.
	NoDeviceSerial,
}

impl From<hid::Error> for Error {
	fn from(e: hid::Error) -> Error {
		Error::Hid(e)
	}
}

impl From<libusb::Error> for Error {
	fn from(e: libusb::Error) -> Error {
		Error::Usb(e)
	}
}

impl error::Error for Error {
	fn cause(&self) -> Option<&error::Error> {
		match *self {
			Error::Hid(ref e) => Some(e),
			Error::Usb(ref e) => Some(e),
			_ => None,
		}
	}

	fn description(&self) -> &str {
		match *self {
			Error::Hid(ref e) => error::Error::description(e),
			Error::Usb(ref e) => error::Error::description(e),
			Error::DeviceNotFound => "the device to connect to was not found",
			Error::DeviceDisconnected => "the device is no longer available",
			Error::UnknownHidVersion => "HID version of the device unknown",
			Error::UnexpectedChunkSizeFromDevice(_) => {
				"the device produced a data chunk of unexpected size"
			}
			Error::DeviceReadTimeout => "timeout expired while reading from device",
			Error::DeviceBadMagic => "the device sent chunk with wrong magic value",
			Error::DeviceBadSessionId => "the device sent a message with a wrong session id",
			Error::DeviceUnexpectedSequenceNumber => {
				"the device sent an unexpected sequence number"
			}
			Error::InvalidMessageType(_) => "received a non-existing message type from the device",
			Error::NoDeviceSerial => "unable to determine device serial number",
		}
	}
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let desc = error::Error::description;
		match *self {
			Error::Hid(ref e) => fmt::Display::fmt(e, f),
			Error::Usb(ref e) => fmt::Display::fmt(e, f),
			Error::UnexpectedChunkSizeFromDevice(s) => write!(f, "{}: {}", desc(self), s),
			Error::InvalidMessageType(ref t) => write!(f, "{}: {}", desc(self), t),
			_ => f.write_str(desc(self)),
		}
	}
}
