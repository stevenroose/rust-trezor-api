use std::fmt;

use protobuf;

use super::Model;
use error::{Error, Result};
use protos::MessageType::{
	MessageType_ButtonRequest, MessageType_Failure, MessageType_PassphraseRequest,
	MessageType_PinMatrixRequest,
};
use protos::{self, MessageType};
use transport::{ProtoMessage, Transport};

#[derive(Clone, Debug)]
pub enum InteractionRequest {
	ButtonRequest(protos::ButtonRequest),
	PinRequest(protos::PinMatrixRequest),
	PassphraseRequest(protos::PassphraseRequest),
}

impl fmt::Display for InteractionRequest {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			InteractionRequest::ButtonRequest(ref m) => write!(f, "ButtonRequest: {:?}", m),
			InteractionRequest::PinRequest(ref m) => write!(f, "PinRequest: {:?}", m),
			InteractionRequest::PassphraseRequest(ref m) => write!(f, "PassphraseRequest: {:?}", m),
		}
	}
}

#[derive(Clone, Debug)]
pub enum TrezorResponse<T: protobuf::Message> {
	Ok(T),
	Failure(protos::Failure),
	Request(InteractionRequest),
}

impl<T: protobuf::Message> fmt::Display for TrezorResponse<T> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			TrezorResponse::Ok(ref m) => write!(f, "Ok: {:?}", m),
			TrezorResponse::Failure(ref m) => write!(f, "Failure: {:?}", m),
			TrezorResponse::Request(ref m) => write!(f, "Request: {}", m),
		}
	}
}

impl<T: protobuf::Message> TrezorResponse<T> {
	pub fn ok(self) -> Result<T> {
		match self {
			TrezorResponse::Ok(m) => Ok(m),
			TrezorResponse::Failure(m) => Err(Error::FailureResponse(m)),
			TrezorResponse::Request(r) => Err(Error::UnexpectedInteractionRequest(r)),
		}
	}
}

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
		mt_send: MessageType,
		mt_recv: MessageType,
		message: S,
	) -> Result<TrezorResponse<R>>
	where
		S: protobuf::Message,
		R: protobuf::Message,
	{
		self.transport.write_message(ProtoMessage(mt_send, message.write_to_bytes()?))?;
		let resp = self.transport.read_message()?;
		// Somehow I can't include mt_recv in the same match block.
		if resp.message_type() == mt_recv {
			Ok(TrezorResponse::Ok(resp.take_message()?))
		} else {
			match resp.message_type() {
				MessageType_Failure => Ok(TrezorResponse::Failure(resp.take_message()?)),
				MessageType_ButtonRequest => Ok(TrezorResponse::Request(
					InteractionRequest::ButtonRequest(resp.take_message()?),
				)),
				MessageType_PinMatrixRequest => Ok(TrezorResponse::Request(
					InteractionRequest::PinRequest(resp.take_message()?),
				)),
				MessageType_PassphraseRequest => Ok(TrezorResponse::Request(
					InteractionRequest::PassphraseRequest(resp.take_message()?),
				)),
				mtype => Err(Error::UnexpectedMessageType(mtype)),
			}
		}
	}

	fn init_device(&mut self) -> Result<()> {
		self.features = Some(self.initialize()?);
		Ok(())
	}
}

pub trait TrezorClient {
	fn call<S, R>(
		&mut self,
		mt_send: MessageType,
		mt_recv: MessageType,
		message: S,
	) -> Result<TrezorResponse<R>>
	where
		S: protobuf::Message,
		R: protobuf::Message;

	fn init_device(&mut self) -> Result<()>;

	//TODO(stevenroose) macronize all the things!

	fn initialize(&mut self) -> Result<protos::Features> {
		let mut req = protos::Initialize::new();
		req.set_state(Vec::new());
		self.call(MessageType::MessageType_Initialize, MessageType::MessageType_Features, req)?.ok()
	}

	fn ping(&mut self, message: &str) -> Result<()> {
		let mut req = protos::Ping::new();
		req.set_message(message.to_owned());
		let _: protos::Success = self
			.call(MessageType::MessageType_Ping, MessageType::MessageType_Success, req)?
			.ok()?;
		Ok(())
	}

	fn change_pin(&mut self, remove: bool) -> Result<protos::ButtonRequest> {
		let mut req = protos::ChangePin::new();
		req.set_remove(remove);
		self.call(MessageType::MessageType_ChangePin, MessageType::MessageType_Success, req)?.ok()
	}

	fn wipe_device(&mut self) -> Result<()> {
		let req = protos::WipeDevice::new();
		let _: protos::Success = self
			.call(MessageType::MessageType_WipeDevice, MessageType::MessageType_Success, req)?
			.ok()?;
		self.init_device()?;
		Ok(())
	}
}
