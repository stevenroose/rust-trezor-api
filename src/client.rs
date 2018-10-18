use protobuf;

use super::Model;
use error::Result;
use protos::{self, MessageType};
use transport::Transport;

pub struct Trezor {
	transport: Box<Transport>,
	pub model: Model,
	// Cached features for later inspection.
	pub features: Option<protos::Features>,
}

impl Trezor {
	pub fn new(model: Model, transport: Box<Transport>) -> Trezor {
		Trezor {
			model: model,
			transport: transport,
			features: None,
		}
	}
}

impl TrezorClient for Trezor {
	#[inline]
	fn call<S, R>(
		&mut self,
		type_send: MessageType,
		type_receive: MessageType,
		message: S,
	) -> Result<R>
	where
		S: protobuf::Message,
		R: protobuf::Message,
	{
		self.transport.write_message(type_send, message.write_to_bytes()?)?;
		Ok(protobuf::parse_from_bytes(&self.transport.read_message(type_receive)?)?)
	}

	fn init_device(&mut self) -> Result<()> {
		self.features = Some(self.initialize()?);
		Ok(())
	}
}

pub trait TrezorClient {
	fn call<S, R>(
		&mut self,
		type_send: MessageType,
		type_receive: MessageType,
		message: S,
	) -> Result<R>
	where
		S: protobuf::Message,
		R: protobuf::Message;

	fn init_device(&mut self) -> Result<()>;

	//TODO(stevenroose) macronize all the things!

	fn initialize(&mut self) -> Result<protos::Features> {
		let mut req = protos::Initialize::new();
		req.set_state(Vec::new());
		self.call(MessageType::MessageType_Initialize, MessageType::MessageType_Features, req)
	}

	fn ping(&mut self, message: &str) -> Result<()> {
		let mut req = protos::Ping::new();
		req.set_message(message.to_owned());
		let _: protos::Success =
			self.call(MessageType::MessageType_Ping, MessageType::MessageType_Success, req)?;
		Ok(())
	}

	fn change_pin(&mut self, remove: bool) -> Result<protos::ButtonRequest> {
		let mut req = protos::ChangePin::new();
		req.set_remove(remove);
		self.call(MessageType::MessageType_ChangePin, MessageType::MessageType_Success, req)
	}

	fn wipe_device(&mut self) -> Result<()> {
		let mut req = protos::WipeDevice::new();
		let _: protos::Success =
			self.call(MessageType::MessageType_WipeDevice, MessageType::MessageType_Success, req)?;
		self.init_device()?;
		Ok(())
	}
}
