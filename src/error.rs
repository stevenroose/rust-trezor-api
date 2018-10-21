//! # Error Handling

use std::error;
use std::fmt;
use std::result;

use client::{Failure, InteractionType};
use hid;
use protobuf::error::ProtobufError;
use protos;

/// Trezor error.
#[derive(Debug)]
pub enum Error {
	/// Error from hidapi.
	Hid(hid::Error),
	/// Less than one device was plugged in.
	NoDeviceFound,
	/// More than one device was plugged in.
	DeviceNotUnique,
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
	/// Received an unexpected message type from the device.
	UnexpectedMessageType(protos::MessageType), //TODO(stevenroose) type alias
	/// Error reading or writing protobuf messages.
	Protobuf(ProtobufError),
	/// A failure message was returned by the device.
	FailureResponse(Failure),
	/// An unexpected interaction request was returned by the device.
	UnexpectedInteractionRequest(InteractionType),
}

impl From<hid::Error> for Error {
	fn from(e: hid::Error) -> Error {
		Error::Hid(e)
	}
}

impl From<ProtobufError> for Error {
	fn from(e: ProtobufError) -> Error {
		Error::Protobuf(e)
	}
}

impl error::Error for Error {
	fn cause(&self) -> Option<&error::Error> {
		match *self {
			Error::Hid(ref e) => Some(e),
			_ => None,
		}
	}

	fn description(&self) -> &str {
		match *self {
			Error::Hid(ref e) => error::Error::description(e),
			Error::NoDeviceFound => "Trezor device not found",
			Error::DeviceNotUnique => "multiple Trezor devices found",
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
			Error::UnexpectedMessageType(_) => {
				"received an unexpected message type from the device"
			}
			Error::Protobuf(_) => "error reading or writing protobuf messages",
			Error::FailureResponse(_) => "a failure message was returned by the device",
			Error::UnexpectedInteractionRequest(_) => {
				"an unexpected interaction request was returned by the device"
			}
		}
	}
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			Error::Hid(ref e) => fmt::Display::fmt(e, f),
			Error::UnexpectedChunkSizeFromDevice(s) => {
				write!(f, "device produced chunk of size {}", s)
			}
			Error::InvalidMessageType(ref t) => write!(f, "received invalid message type: {}", t),
			Error::UnexpectedMessageType(ref t) => {
				write!(f, "received unexpected message type: {:?}", t)
			}
			Error::Protobuf(ref e) => write!(f, "protobuf: {}", e),
			Error::FailureResponse(ref e) => write!(
				f,
				r#"failure received: code={:?} message="{}""#,
				e.get_code(),
				e.get_message()
			),
			Error::UnexpectedInteractionRequest(ref r) => {
				write!(f, "unexpected interaction request: {:?}", r)
			}
			_ => f.write_str(error::Error::description(self)),
		}
	}
}

/// Result type used in this crate.
pub type Result<T> = result::Result<T, Error>;
