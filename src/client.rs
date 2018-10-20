use std::fmt;

use protobuf;

use super::Model;
use error::{Error, Result};
use protos::MessageType::*;
use protos::{self, MessageType};
use transport::{ProtoMessage, Transport};

pub trait TrezorMessage: protobuf::Message {
	fn message_type() -> MessageType;
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum InteractionType {
	Button,
	PinMatrix,
	Passphrase,
}

pub struct ButtonRequest<T: TrezorMessage> {
	pub message: protos::ButtonRequest,
	next: Box<Fn() -> Result<TrezorResponse<T>>>,
}

impl<T: TrezorMessage> fmt::Debug for ButtonRequest<T> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		fmt::Debug::fmt(&self.message, f)
	}
}

impl<T: TrezorMessage> ButtonRequest<T> {
	pub fn ack(self) -> Result<TrezorResponse<T>> {
		let n = self.next;
		n()
	}
}

pub struct PinMatrixRequest<T: TrezorMessage> {
	pub message: protos::PinMatrixRequest,
	next: Box<Fn(String) -> Result<TrezorResponse<T>>>,
}

impl<T: TrezorMessage> fmt::Debug for PinMatrixRequest<T> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		fmt::Debug::fmt(&self.message, f)
	}
}

impl<T: TrezorMessage> PinMatrixRequest<T> {
	pub fn ack(self, pin: String) -> Result<TrezorResponse<T>> {
		let n = self.next;
		n(pin)
	}
}

pub struct PassphraseRequest<T: TrezorMessage> {
	pub message: protos::PassphraseRequest,
	next: Box<Fn(String) -> Result<TrezorResponse<T>>>,
}

impl<T: TrezorMessage> fmt::Debug for PassphraseRequest<T> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		fmt::Debug::fmt(&self.message, f)
	}
}

impl<T: TrezorMessage> PassphraseRequest<T> {
	pub fn ack(self, passphrase: String) -> Result<TrezorResponse<T>> {
		let n = self.next;
		n(passphrase)
	}
}

#[derive(Debug)]
pub enum TrezorResponse<T: TrezorMessage> {
	Ok(T),
	Failure(protos::Failure),
	ButtonRequest(ButtonRequest<T>),
	PinMatrixRequest(PinMatrixRequest<T>),
	PassphraseRequest(PassphraseRequest<T>),
}

impl<T: TrezorMessage> fmt::Display for TrezorResponse<T> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			TrezorResponse::Ok(ref m) => write!(f, "Ok: {:?}", m),
			TrezorResponse::Failure(ref m) => write!(f, "Failure: {:?}", m),
			TrezorResponse::ButtonRequest(ref r) => write!(f, "ButtonRequest: {:?}", r),
			TrezorResponse::PinMatrixRequest(ref r) => write!(f, "PinMatrixRequest: {:?}", r),
			TrezorResponse::PassphraseRequest(ref r) => write!(f, "PassphraseRequest: {:?}", r),
		}
	}
}

impl<T: TrezorMessage> TrezorResponse<T> {
	pub fn ok(self) -> Result<T> {
		match self {
			TrezorResponse::Ok(m) => Ok(m),
			TrezorResponse::Failure(m) => Err(Error::FailureResponse(m)),
			TrezorResponse::ButtonRequest(r) => {
				Err(Error::UnexpectedInteractionRequest(InteractionType::Button))
			}
			TrezorResponse::PinMatrixRequest(r) => {
				Err(Error::UnexpectedInteractionRequest(InteractionType::PinMatrix))
			}
			TrezorResponse::PassphraseRequest(r) => {
				Err(Error::UnexpectedInteractionRequest(InteractionType::Passphrase))
			}
		}
	}

	pub fn button_request(self) -> Result<ButtonRequest<T>> {
		match self {
			TrezorResponse::ButtonRequest(r) => Ok(r),
			TrezorResponse::Ok(m) => Err(Error::UnexpectedMessageType(T::message_type())),
			TrezorResponse::Failure(m) => Err(Error::FailureResponse(m)),
			TrezorResponse::PinMatrixRequest(r) => {
				Err(Error::UnexpectedInteractionRequest(InteractionType::PinMatrix))
			}
			TrezorResponse::PassphraseRequest(r) => {
				Err(Error::UnexpectedInteractionRequest(InteractionType::Passphrase))
			}
		}
	}

	pub fn pin_request(self) -> Result<PinMatrixRequest<T>> {
		match self {
			TrezorResponse::PinMatrixRequest(r) => Ok(r),
			TrezorResponse::Ok(m) => Err(Error::UnexpectedMessageType(T::message_type())),
			TrezorResponse::Failure(m) => Err(Error::FailureResponse(m)),
			TrezorResponse::ButtonRequest(r) => {
				Err(Error::UnexpectedInteractionRequest(InteractionType::Button))
			}
			TrezorResponse::PassphraseRequest(r) => {
				Err(Error::UnexpectedInteractionRequest(InteractionType::Passphrase))
			}
		}
	}

	pub fn passphrase_request(self) -> Result<PassphraseRequest<T>> {
		match self {
			TrezorResponse::PassphraseRequest(r) => Ok(r),
			TrezorResponse::Ok(m) => Err(Error::UnexpectedMessageType(T::message_type())),
			TrezorResponse::Failure(m) => Err(Error::FailureResponse(m)),
			TrezorResponse::ButtonRequest(r) => {
				Err(Error::UnexpectedInteractionRequest(InteractionType::Button))
			}
			TrezorResponse::PinMatrixRequest(r) => {
				Err(Error::UnexpectedInteractionRequest(InteractionType::PinMatrix))
			}
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
	fn call<S, R>(&mut self, message: S) -> Result<TrezorResponse<R>>
	where
		S: TrezorMessage,
		R: TrezorMessage,
	{
		self.transport.write_message(ProtoMessage(S::message_type(), message.write_to_bytes()?))?;
		let resp = self.transport.read_message()?;
		if resp.message_type() == R::message_type() {
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
	fn call<S, R>(&mut self, message: S) -> Result<TrezorResponse<R>>
	where
		S: TrezorMessage,
		R: TrezorMessage;

	fn init_device(&mut self) -> Result<()>;

	//TODO(stevenroose) macronize all the things!

	fn initialize(&mut self) -> Result<protos::Features> {
		let mut req = protos::Initialize::new();
		req.set_state(Vec::new());
		self.call(req)?.ok()
	}

	fn ping(&mut self, message: &str) -> Result<()> {
		let mut req = protos::Ping::new();
		req.set_message(message.to_owned());
		let _: protos::Success = self.call(req)?.ok()?;
		Ok(())
	}

	fn change_pin(&mut self, remove: bool) -> Result<protos::ButtonRequest> {
		let mut req = protos::ChangePin::new();
		req.set_remove(remove);
		self.call(req)?.ok()
	}

	fn wipe_device(&mut self) -> Result<()> {
		let req = protos::WipeDevice::new();
		let _: protos::Success = self.call(req)?.ok()?;
		self.init_device()?;
		Ok(())
	}

	//TODO(stevenroose) fill gap

	fn pin_matrix_ack(&mut self, pin: String) -> Result<()> {
		let mut req = protos::PinMatrixAck::new();
		req.set_pin(pin);
		let _: protos::Success = self.call(req)?.ok()?;
		Ok(())
	}

	//TODO(stevenroose) fill gap

	fn button_ack<R: TrezorMessage>(&mut self) -> Result<R> {
		let req = protos::ButtonAck::new();
		self.call(req)?.ok()
	}
}
