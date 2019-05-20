//! # Error Handling

use std::error;
use std::fmt;
use std::result;

use bitcoin;
use bitcoin::util::base58;
use bitcoin_hashes::sha256d;
use protobuf::error::ProtobufError;
use secp256k1;

use client::InteractionType;
use protos;
use transport;

/// Trezor error.
#[derive(Debug)]
pub enum Error {
	/// Less than one device was plugged in.
	NoDeviceFound,
	/// More than one device was plugged in.
	DeviceNotUnique,
	/// Transport error connecting to device.
	TransportConnect(transport::error::Error),
	/// Transport error while beginning a session.
	TransportBeginSession(transport::error::Error),
	/// Transport error while ending a session.
	TransportEndSession(transport::error::Error),
	/// Transport error while sending a message.
	TransportSendMessage(transport::error::Error),
	/// Transport error while receiving a message.
	TransportReceiveMessage(transport::error::Error),
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
	TxRequestUnknownTxid(sha256d::Hash),
	/// The PSBT is missing the full tx for given input.
	PsbtMissingInputTx(sha256d::Hash),
	/// Device produced invalid TxRequest message.
	MalformedTxRequest(protos::TxRequest),
	/// User provided invalid PSBT.
	InvalidPsbt(String),
	/// Error encoding/decoding a Bitcoin data structure.
	BitcoinEncode(bitcoin::consensus::encode::Error),
	/// Elliptic curve crypto error.
	Secp256k1(secp256k1::Error),
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
			Error::TransportConnect(ref e) => Some(e),
			Error::TransportBeginSession(ref e) => Some(e),
			Error::TransportEndSession(ref e) => Some(e),
			Error::TransportSendMessage(ref e) => Some(e),
			Error::TransportReceiveMessage(ref e) => Some(e),
			Error::Base58(ref e) => Some(e),
			_ => None,
		}
	}

	fn description(&self) -> &str {
		match *self {
			Error::NoDeviceFound => "Trezor device not found",
			Error::DeviceNotUnique => "multiple Trezor devices found",
			Error::TransportConnect(_) => "transport error connecting to device",
			Error::TransportBeginSession(_) => "transport error while beginning a session",
			Error::TransportEndSession(_) => "transport error while ending a session",
			Error::TransportSendMessage(_) => "transport error while sending a message",
			Error::TransportReceiveMessage(_) => "transport error while receiving a message",
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
			Error::InvalidEntropy => "provided entropy is not 32 bytes",
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
			Error::TransportConnect(ref e) => write!(f, "transport connect: {}", e),
			Error::TransportBeginSession(ref e) => write!(f, "transport beginning session: {}", e),
			Error::TransportEndSession(ref e) => write!(f, "transport ending session: {}", e),
			Error::TransportSendMessage(ref e) => write!(f, "transport sending message: {}", e),
			Error::TransportReceiveMessage(ref e) => {
				write!(f, "transport receiving message: {}", e)
			}
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
