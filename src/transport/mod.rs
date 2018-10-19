use fmt;
use protobuf;

use super::AvailableDevice;
use error::Result;
use protos::MessageType;

pub mod hid;
pub mod protocol;

//mod messages;
//pub use self::messages::ProtoMessage;

#[derive(Debug)]
pub enum AvailableDeviceTransport {
	Hid(hid::AvailableHidTransport),
}

impl fmt::Display for AvailableDeviceTransport {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			AvailableDeviceTransport::Hid(ref t) => write!(f, "{}", t),
		}
	}
}

pub struct ProtoMessage(pub MessageType, pub Vec<u8>);

impl ProtoMessage {
	pub fn new(mt: MessageType, payload: Vec<u8>) -> ProtoMessage {
		ProtoMessage(mt, payload)
	}
	pub fn message_type(&self) -> MessageType {
		self.0
	}
	pub fn payload(&self) -> &[u8] {
		&self.1
	}
	pub fn take_payload(self) -> Vec<u8> {
		self.1
	}

	pub fn take_message<M: protobuf::Message>(self) -> Result<M> {
		Ok(protobuf::parse_from_bytes(&self.payload())?)
	}
}

pub trait Transport {
	fn session_begin(&mut self) -> Result<()>;
	fn session_end(&mut self) -> Result<()>;

	fn write_message(&mut self, message: ProtoMessage) -> Result<()>;
	fn read_message(&mut self) -> Result<ProtoMessage>;
}

pub fn connect(available_device: &AvailableDevice) -> Result<Box<Transport>> {
	match available_device.transport {
		AvailableDeviceTransport::Hid(_) => hid::HidTransport::connect(&available_device),
	}
}
