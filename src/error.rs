//! # Error Handling

use std::error;
use std::fmt;
use std::result;

use bitcoin;
use bitcoin::util::base58;
use bitcoin::util::hash::Sha256dHash;
use hid;
use protobuf::error::ProtobufError;
use secp256k1;

use client::InteractionType;
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
	FailureResponse(protos::Failure),
	/// An unexpected interaction request was returned by the device.
	UnexpectedInteractionRequest(InteractionType),
	/// Error in Base58 decoding
	Base58(base58::Error),
	/// The given Bitcoin network is not supported.
	UnsupportedNetwork,
	/// Provided entropy is not 32 bytes.
	InvalidEntropy,
	/// The device referenced a non-existing input or output index.
	TxRequestInvalidIndex(usize),
	/// The device referenced an unknown TXID.
	TxRequestUnknownTxid(Sha256dHash),
	/// The PSBT is missing the full tx for given input.
	PsbtMissingInputTx(Sha256dHash),
	/// Device produced invalid TxRequest message.
	MalformedTxRequest(protos::TxRequest),
	/// User provided invalid PSBT.
	InvalidPsbt(String),
	/// Error encoding/decoding a Bitcoin data structure.
	BitcoinEncode(bitcoin::consensus::encode::Error),
	/// Elliptic curve crypto error.
	Secp256k1(secp256k1::Error),
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

impl From<base58::Error> for Error {
	fn from(e: base58::Error) -> Error {
		Error::Base58(e)
	}
}

impl From<bitcoin::consensus::encode::Error> for Error {
	fn from(e: bitcoin::consensus::encode::Error) -> Error {
		Error::BitcoinEncode(e)
	}
}

impl From<secp256k1::Error> for Error {
	fn from(e: secp256k1::Error) -> Error {
		Error::Secp256k1(e)
	}
}

impl error::Error for Error {
	fn cause(&self) -> Option<&error::Error> {
		match *self {
			Error::Hid(ref e) => Some(e),
			Error::Base58(ref e) => Some(e),
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
			Error::Base58(ref e) => error::Error::description(e),
			Error::UnsupportedNetwork => "given network is not supported",
			Error::InvalidEntropy = "provided entropy is not 32 bytes",
			Error::TxRequestInvalidIndex(_) => {
				"the device referenced a non-existing input or output index"
			}
			Error::TxRequestUnknownTxid(_) => "the device referenced an unknown TXID",
			Error::PsbtMissingInputTx(_) => "the PSBT is missing the full tx for given input",
			Error::MalformedTxRequest(_) => "device produced invalid TxRequest message",
			Error::InvalidPsbt(_) => "user provided invalid PSBT",
			Error::BitcoinEncode(_) => "error encoding/decoding a Bitcoin data structure",
			Error::Secp256k1(_) => "elliptic curve crypto error",
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
			Error::Base58(ref e) => fmt::Display::fmt(e, f),
			Error::TxRequestInvalidIndex(ref i) => {
				write!(f, "device referenced non-existing input or output index: {}", i)
			}
			Error::TxRequestUnknownTxid(ref txid) => {
				write!(f, "device referenced unknown TXID: {}", txid)
			}
			Error::PsbtMissingInputTx(ref txid) => write!(f, "PSBT missing input tx: {}", txid),
			Error::MalformedTxRequest(ref m) => write!(f, "malformed TxRequest: {:?}", m),
			Error::InvalidPsbt(ref m) => write!(f, "invalid PSBT: {}", m),
			Error::BitcoinEncode(ref e) => write!(f, "bitcoin encoding error: {}", e),
			Error::Secp256k1(ref e) => write!(f, "ECDSA signature error: {}", e),
			_ => f.write_str(error::Error::description(self)),
		}
	}
}

/// Result type used in this crate.
pub type Result<T> = result::Result<T, Error>;
