use std::fmt;
use std::marker::PhantomData;

use super::Model;
use error::{Error, Result};
use messages::TrezorMessage;
use protos;
use protos::MessageType::*;
use transport::{ProtoMessage, Transport};

// Some types with raw protos that we use in the public interface so they have to be exported.
pub type Success = protos::Success;
pub type Failure = protos::Failure;
pub type Features = protos::Features;
pub type ButtonRequestType = protos::ButtonRequest_ButtonRequestType;
pub type PinMatrixRequestType = protos::PinMatrixRequest_PinMatrixRequestType;

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum InteractionType {
	Button,
	PinMatrix,
	Passphrase,
}

pub struct ButtonRequest<'a, T: TrezorMessage> {
	message: protos::ButtonRequest,
	client: &'a mut Trezor,
	_result_type: PhantomData<T>,
}

impl<'a, T: TrezorMessage> fmt::Debug for ButtonRequest<'a, T> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		fmt::Debug::fmt(&self.message, f)
	}
}

impl<'a, T: TrezorMessage> ButtonRequest<'a, T> {
	pub fn request_type(&self) -> ButtonRequestType {
		self.message.get_code()
	}

	pub fn request_data(&self) -> &str {
		self.message.get_data()
	}

	pub fn ack(self) -> Result<TrezorResponse<'a, T>> {
		let req = protos::ButtonAck::new();
		self.client.call(req)
	}
}

pub struct PinMatrixRequest<'a, T: TrezorMessage> {
	message: protos::PinMatrixRequest,
	client: &'a mut Trezor,
	_result_type: PhantomData<T>,
}

impl<'a, T: TrezorMessage> fmt::Debug for PinMatrixRequest<'a, T> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		fmt::Debug::fmt(&self.message, f)
	}
}

impl<'a, T: TrezorMessage> PinMatrixRequest<'a, T> {
	pub fn request_type(&self) -> PinMatrixRequestType {
		self.message.get_field_type()
	}

	pub fn ack_pin(self, pin: String) -> Result<TrezorResponse<'a, T>> {
		let mut req = protos::PinMatrixAck::new();
		req.set_pin(pin);
		self.client.call(req)
	}

	pub fn ack(self) -> Result<TrezorResponse<'a, T>> {
		let req = protos::PinMatrixAck::new();
		self.client.call(req)
	}
}

pub struct PassphraseRequest<'a, T: TrezorMessage> {
	message: protos::PassphraseRequest,
	client: &'a mut Trezor,
	_result_type: PhantomData<T>,
}

impl<'a, T: TrezorMessage> fmt::Debug for PassphraseRequest<'a, T> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		fmt::Debug::fmt(&self.message, f)
	}
}

impl<'a, T: TrezorMessage> PassphraseRequest<'a, T> {
	pub fn on_device(&self) -> bool {
		self.message.get_on_device()
	}

	pub fn ack_passphrase(self, passphrase: String) -> Result<TrezorResponse<'a, T>> {
		let mut req = protos::PassphraseAck::new();
		req.set_passphrase(passphrase);
		self.client.call(req)
	}
}

#[derive(Debug)]
pub enum TrezorResponse<'a, T: TrezorMessage> {
	Ok(T),
	Failure(Failure),
	ButtonRequest(ButtonRequest<'a, T>),
	PinMatrixRequest(PinMatrixRequest<'a, T>),
	PassphraseRequest(PassphraseRequest<'a, T>),
}

impl<'a, T: TrezorMessage> fmt::Display for TrezorResponse<'a, T> {
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

impl<'a, T: TrezorMessage> TrezorResponse<'a, T> {
	pub fn ok(self) -> Result<T> {
		match self {
			TrezorResponse::Ok(m) => Ok(m),
			TrezorResponse::Failure(m) => Err(Error::FailureResponse(m)),
			TrezorResponse::ButtonRequest(_) => {
				Err(Error::UnexpectedInteractionRequest(InteractionType::Button))
			}
			TrezorResponse::PinMatrixRequest(_) => {
				Err(Error::UnexpectedInteractionRequest(InteractionType::PinMatrix))
			}
			TrezorResponse::PassphraseRequest(_) => {
				Err(Error::UnexpectedInteractionRequest(InteractionType::Passphrase))
			}
		}
	}

	pub fn button_request(self) -> Result<ButtonRequest<'a, T>> {
		match self {
			TrezorResponse::ButtonRequest(r) => Ok(r),
			TrezorResponse::Ok(_) => Err(Error::UnexpectedMessageType(T::message_type())),
			TrezorResponse::Failure(m) => Err(Error::FailureResponse(m)),
			TrezorResponse::PinMatrixRequest(_) => {
				Err(Error::UnexpectedInteractionRequest(InteractionType::PinMatrix))
			}
			TrezorResponse::PassphraseRequest(_) => {
				Err(Error::UnexpectedInteractionRequest(InteractionType::Passphrase))
			}
		}
	}

	pub fn pin_matrix_request(self) -> Result<PinMatrixRequest<'a, T>> {
		match self {
			TrezorResponse::PinMatrixRequest(r) => Ok(r),
			TrezorResponse::Ok(_) => Err(Error::UnexpectedMessageType(T::message_type())),
			TrezorResponse::Failure(m) => Err(Error::FailureResponse(m)),
			TrezorResponse::ButtonRequest(_) => {
				Err(Error::UnexpectedInteractionRequest(InteractionType::Button))
			}
			TrezorResponse::PassphraseRequest(_) => {
				Err(Error::UnexpectedInteractionRequest(InteractionType::Passphrase))
			}
		}
	}

	pub fn passphrase_request(self) -> Result<PassphraseRequest<'a, T>> {
		match self {
			TrezorResponse::PassphraseRequest(r) => Ok(r),
			TrezorResponse::Ok(_) => Err(Error::UnexpectedMessageType(T::message_type())),
			TrezorResponse::Failure(m) => Err(Error::FailureResponse(m)),
			TrezorResponse::ButtonRequest(_) => {
				Err(Error::UnexpectedInteractionRequest(InteractionType::Button))
			}
			TrezorResponse::PinMatrixRequest(_) => {
				Err(Error::UnexpectedInteractionRequest(InteractionType::PinMatrix))
			}
		}
	}
}

pub struct Trezor {
	transport: Box<Transport>,
	pub model: Model,
	// Cached features for later inspection.
	pub features: Option<Features>,
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

impl Trezor {
	pub fn call<'a, S, R>(&'a mut self, message: S) -> Result<TrezorResponse<'a, R>>
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
				MessageType_ButtonRequest => Ok(TrezorResponse::ButtonRequest(ButtonRequest {
					message: resp.take_message()?,
					client: self,
					_result_type: PhantomData,
				})),
				MessageType_PinMatrixRequest => {
					Ok(TrezorResponse::PinMatrixRequest(PinMatrixRequest {
						message: resp.take_message()?,
						client: self,
						_result_type: PhantomData,
					}))
				}
				MessageType_PassphraseRequest => {
					Ok(TrezorResponse::PassphraseRequest(PassphraseRequest {
						message: resp.take_message()?,
						client: self,
						_result_type: PhantomData,
					}))
				}
				mtype => Err(Error::UnexpectedMessageType(mtype)),
			}
		}
	}

	pub fn init_device(&mut self) -> Result<()> {
		self.features = Some(self.initialize()?);
		Ok(())
	}

	//TODO(stevenroose) macronize all the things!

	pub fn initialize(&mut self) -> Result<Features> {
		let mut req = protos::Initialize::new();
		req.set_state(Vec::new());
		self.call(req)?.ok()
	}

	pub fn ping(&mut self, message: &str) -> Result<()> {
		let mut req = protos::Ping::new();
		req.set_message(message.to_owned());
		let _: protos::Success = self.call(req)?.ok()?;
		Ok(())
	}

	pub fn change_pin(&mut self, remove: bool) -> Result<TrezorResponse<Success>> {
		let mut req = protos::ChangePin::new();
		req.set_remove(remove);
		self.call(req)
	}

	pub fn wipe_device(&mut self) -> Result<()> {
		let req = protos::WipeDevice::new();
		let _: protos::Success = self.call(req)?.ok()?;
		self.init_device()?;
		Ok(())
	}
}
