//! # Error Handling

use std::result;

use bitcoin::util::base58;
use hid;
use protobuf::error::ProtobufError;
use secp256k1;
use std::{error, fmt, io, string};

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
	/// The device sent an unexpected response message.
	DeviceUnexpectedMessageType,
	/// The device sent an unexpected sequence number.
	DeviceUnexpectedSequenceNumber,
	/// Error reading or writing protobuf messages.
	Protobuf(ProtobufError),

	// unused:
	/// Error in Base58 decoding
	Base58(base58::Error),
	/// std io error
	Io(io::Error),
	/// Error from libsecp
	Secp(secp256k1::Error),
	/// Error parsing text
	Utf8(string::FromUtf8Error),
	/// APDU reply had bad status word
	ApduBadStatus(u16),
	/// APDU reply had wrong channel
	ApduWrongChannel,
	/// APDU reply had wrong tag
	ApduWrongTag,
	/// APDU reply had out of order sequence numbers
	ApduWrongSequence,
	/// Received message with invalid length (message, received length)
	ResponseWrongLength(u8, usize),
	/// An wallet does not have enough money (had, required)
	InsufficientFunds(u64, u64),
	/// An wallet cannot produce anymore addresses
	WalletFull,
	/// An encrypted wallet had a bad filesize
	WalletWrongSize(usize),
	/// An encrypted wallet had a bad magic (probably not a wallet)
	WalletWrongMagic(u64),
	/// Attempted to use a user ID that exceeds the field length of the wallet (used, max)
	UserIdTooLong(usize, usize),
	/// Attempted to use a note that exceeds the field length of the wallet (used, max)
	NoteTooLong(usize, usize),
	/// Tried to access entry not in the wallet
	EntryOutOfRange(usize),
	/// Searched for an address not in the wallet
	AddressNotFound,
	/// Attempted to receive twice to one address
	DoubleReceive,
	/// Received an unparseable signature
	BadSignature,
	/// The dongle requested we do something unsupported
	Unsupported,
	/// Received APDU frame of shorter than expected length
	UnexpectedEof,
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

// unused:

impl From<base58::Error> for Error {
	fn from(e: base58::Error) -> Error {
		Error::Base58(e)
	}
}

impl From<io::Error> for Error {
	fn from(e: io::Error) -> Error {
		Error::Io(e)
	}
}

impl From<secp256k1::Error> for Error {
	fn from(e: secp256k1::Error) -> Error {
		Error::Secp(e)
	}
}

impl From<string::FromUtf8Error> for Error {
	fn from(e: string::FromUtf8Error) -> Error {
		Error::Utf8(e)
	}
}

impl error::Error for Error {
	fn cause(&self) -> Option<&error::Error> {
		match *self {
			Error::Hid(ref e) => Some(e),
			// unused:
			Error::Base58(ref e) => Some(e),
			Error::Io(ref e) => Some(e),
			Error::Secp(ref e) => Some(e),
			Error::Utf8(ref e) => Some(e),
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
			Error::DeviceUnexpectedMessageType => "the device sent an unexpected response message",
			Error::DeviceUnexpectedSequenceNumber => {
				"the device sent an unexpected sequence number"
			}
			Error::Protobuf(_) => "error reading or writing protobuf messages",
			//
			// unused:
			Error::Base58(ref e) => error::Error::description(e),
			Error::Io(ref e) => error::Error::description(e),
			Error::Secp(ref e) => error::Error::description(e),
			Error::Utf8(ref e) => error::Error::description(e),
			Error::ApduBadStatus(_) => "bad APDU status word (is device unlocked?)",
			Error::ApduWrongChannel => "wrong APDU channel (is device running the right app?)",
			Error::ApduWrongTag => "wrong APDU tag (is device running the right app?)",
			Error::ApduWrongSequence => "bad APDU sequence no",
			Error::ResponseWrongLength(_, _) => "bad message length",
			Error::InsufficientFunds(_, _) => "insufficient funds",
			Error::WalletFull => "wallet is full, it has no more available addresses",
			Error::WalletWrongSize(_) => "wallet had invalid length",
			Error::WalletWrongMagic(_) => "wallet had wrong magic",
			Error::UserIdTooLong(_, _) => "user ID too long",
			Error::NoteTooLong(_, _) => "note too long",
			Error::EntryOutOfRange(_) => "tried to access entry outside of wallet",
			Error::AddressNotFound => "address not found in wallet",
			Error::DoubleReceive => "attempted to receive twice to same address",
			Error::BadSignature => "unparseable signature",
			Error::Unsupported => "we were asked to do something unsupported",
			Error::UnexpectedEof => "unexpected end of data",
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
			Error::Protobuf(ref e) => write!(f, "protobuf: {}", e),
			_ => f.write_str(error::Error::description(self)),
			//
			// unused:
			Error::Base58(ref e) => fmt::Display::fmt(e, f),
			Error::Io(ref e) => fmt::Display::fmt(e, f),
			Error::Secp(ref e) => fmt::Display::fmt(e, f),
			Error::Utf8(ref e) => fmt::Display::fmt(e, f),
			Error::ApduBadStatus(sw) => write!(f, "bad APDU status word {}", sw),
			Error::ResponseWrongLength(msg, len) => {
				write!(f, "bad APDU response length {} for message 0x{:02x}", len, msg)
			}
			Error::InsufficientFunds(had, required) => {
				write!(f, "have {} but need {} satoshi to fund this transaction", had, required)
			}
			Error::WalletWrongSize(len) => write!(f, "bad wallet size {}", len),
			Error::WalletWrongMagic(magic) => write!(f, "bad wallet magic {:08x}", magic),
			Error::UserIdTooLong(used, max) => {
				write!(f, "user ID length {} exceeds max {}", used, max)
			}
			Error::NoteTooLong(used, max) => {
				write!(f, "user ID length {} exceeds max {}", used, max)
			}
			Error::EntryOutOfRange(entry) => write!(f, "entry {} not in wallet", entry),
		}
	}
}

/// Result type used in this crate.
pub type Result<T> = result::Result<T, Error>;
