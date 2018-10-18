use fmt;

use super::AvailableDevice;
use error::Result;
use protos::MessageType;

pub mod hid;

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

pub trait Transport {
	fn session_begin(&mut self) -> Result<()>;
	fn session_end(&mut self) -> Result<()>;

	fn write_message(&mut self, mtype: MessageType, message: Vec<u8>) -> Result<()>;
	fn read_message(&mut self, mtype: MessageType) -> Result<Vec<u8>>;
}

pub fn connect(available_device: &AvailableDevice) -> Result<Box<Transport>> {
	match available_device.transport {
		AvailableDeviceTransport::Hid(_) => hid::HidTransport::connect(&available_device),
	}
}
