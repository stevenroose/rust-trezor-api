use std::fmt;

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
pub type PassphraseSource = protos::ApplySettings_PassphraseSourceType;
pub type InputScriptType = protos::InputScriptType;
pub type PublicKey = protos::PublicKey;

pub enum WordCount {
	W12 = 12,
	W18 = 18,
	W24 = 24,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum InteractionType {
	Button,
	PinMatrix,
	Passphrase,
}

pub struct ButtonRequest<'a, T, R: TrezorMessage> {
	message: protos::ButtonRequest,
	client: &'a mut Trezor,
	result_handler: Box<ResultHandler<T, R>>,
}

impl<'a, T, R: TrezorMessage> fmt::Debug for ButtonRequest<'a, T, R> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		fmt::Debug::fmt(&self.message, f)
	}
}

impl<'a, T, R: TrezorMessage> ButtonRequest<'a, T, R> {
	pub fn request_type(&self) -> ButtonRequestType {
		self.message.get_code()
	}

	pub fn request_data(&self) -> &str {
		self.message.get_data()
	}

	pub fn ack(self) -> Result<TrezorResponse<'a, T, R>> {
		let req = protos::ButtonAck::new();
		self.client.call(req, self.result_handler)
	}
}

pub struct PinMatrixRequest<'a, T, R: TrezorMessage> {
	message: protos::PinMatrixRequest,
	client: &'a mut Trezor,
	result_handler: Box<ResultHandler<T, R>>,
}

impl<'a, T, R: TrezorMessage> fmt::Debug for PinMatrixRequest<'a, T, R> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		fmt::Debug::fmt(&self.message, f)
	}
}

impl<'a, T, R: TrezorMessage> PinMatrixRequest<'a, T, R> {
	pub fn request_type(&self) -> PinMatrixRequestType {
		self.message.get_field_type()
	}

	pub fn ack_pin(self, pin: String) -> Result<TrezorResponse<'a, T, R>> {
		let mut req = protos::PinMatrixAck::new();
		req.set_pin(pin);
		self.client.call(req, self.result_handler)
	}

	pub fn ack(self) -> Result<TrezorResponse<'a, T, R>> {
		let req = protos::PinMatrixAck::new();
		self.client.call(req, self.result_handler)
	}
}

pub struct PassphraseRequest<'a, T, R: TrezorMessage> {
	message: protos::PassphraseRequest,
	client: &'a mut Trezor,
	result_handler: Box<ResultHandler<T, R>>,
}

impl<'a, T, R: TrezorMessage> fmt::Debug for PassphraseRequest<'a, T, R> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		fmt::Debug::fmt(&self.message, f)
	}
}

impl<'a, T, R: TrezorMessage> PassphraseRequest<'a, T, R> {
	pub fn on_device(&self) -> bool {
		self.message.get_on_device()
	}

	pub fn ack_passphrase(self, passphrase: String) -> Result<TrezorResponse<'a, T, R>> {
		let mut req = protos::PassphraseAck::new();
		req.set_passphrase(passphrase);
		self.client.call(req, self.result_handler)
	}
}

#[derive(Debug)]
pub enum TrezorResponse<'a, T, R: TrezorMessage> {
	Ok(T),
	Failure(Failure),
	ButtonRequest(ButtonRequest<'a, T, R>),
	PinMatrixRequest(PinMatrixRequest<'a, T, R>),
	PassphraseRequest(PassphraseRequest<'a, T, R>),
}

impl<'a, T, R: TrezorMessage> fmt::Display for TrezorResponse<'a, T, R> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			TrezorResponse::Ok(ref _m) => write!(f, "Ok"), //TODO(stevenroose) should we make T: Debug?
			TrezorResponse::Failure(ref m) => write!(f, "Failure: {:?}", m),
			TrezorResponse::ButtonRequest(ref r) => write!(f, "ButtonRequest: {:?}", r),
			TrezorResponse::PinMatrixRequest(ref r) => write!(f, "PinMatrixRequest: {:?}", r),
			TrezorResponse::PassphraseRequest(ref r) => write!(f, "PassphraseRequest: {:?}", r),
		}
	}
}

impl<'a, T, R: TrezorMessage> TrezorResponse<'a, T, R> {
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

	pub fn button_request(self) -> Result<ButtonRequest<'a, T, R>> {
		match self {
			TrezorResponse::ButtonRequest(r) => Ok(r),
			TrezorResponse::Ok(_) => Err(Error::UnexpectedMessageType(R::message_type())),
			TrezorResponse::Failure(m) => Err(Error::FailureResponse(m)),
			TrezorResponse::PinMatrixRequest(_) => {
				Err(Error::UnexpectedInteractionRequest(InteractionType::PinMatrix))
			}
			TrezorResponse::PassphraseRequest(_) => {
				Err(Error::UnexpectedInteractionRequest(InteractionType::Passphrase))
			}
		}
	}

	pub fn pin_matrix_request(self) -> Result<PinMatrixRequest<'a, T, R>> {
		match self {
			TrezorResponse::PinMatrixRequest(r) => Ok(r),
			TrezorResponse::Ok(_) => Err(Error::UnexpectedMessageType(R::message_type())),
			TrezorResponse::Failure(m) => Err(Error::FailureResponse(m)),
			TrezorResponse::ButtonRequest(_) => {
				Err(Error::UnexpectedInteractionRequest(InteractionType::Button))
			}
			TrezorResponse::PassphraseRequest(_) => {
				Err(Error::UnexpectedInteractionRequest(InteractionType::Passphrase))
			}
		}
	}

	pub fn passphrase_request(self) -> Result<PassphraseRequest<'a, T, R>> {
		match self {
			TrezorResponse::PassphraseRequest(r) => Ok(r),
			TrezorResponse::Ok(_) => Err(Error::UnexpectedMessageType(R::message_type())),
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

//TODO(stevenroose) should this be FnOnce and put in an FnBox?
type ResultHandler<T, R> = Fn(R) -> Result<T>;

impl Trezor {
	pub fn call_raw<S>(&mut self, message: S) -> Result<ProtoMessage>
	where
		S: TrezorMessage,
	{
		self.transport.write_message(ProtoMessage(S::message_type(), message.write_to_bytes()?))?;
		self.transport.read_message()
	}

	pub fn call<'a, T, S, R>(
		&'a mut self,
		message: S,
		result_handler: Box<ResultHandler<T, R>>,
	) -> Result<TrezorResponse<'a, T, R>>
	where
		S: TrezorMessage,
		R: TrezorMessage,
	{
		let resp = self.call_raw(message)?;
		if resp.message_type() == R::message_type() {
			Ok(TrezorResponse::Ok(result_handler(resp.take_message()?)?))
		} else {
			match resp.message_type() {
				MessageType_Failure => Ok(TrezorResponse::Failure(resp.take_message()?)),
				MessageType_ButtonRequest => Ok(TrezorResponse::ButtonRequest(ButtonRequest {
					message: resp.take_message()?,
					client: self,
					result_handler: result_handler,
				})),
				MessageType_PinMatrixRequest => {
					Ok(TrezorResponse::PinMatrixRequest(PinMatrixRequest {
						message: resp.take_message()?,
						client: self,
						result_handler: result_handler,
					}))
				}
				MessageType_PassphraseRequest => {
					Ok(TrezorResponse::PassphraseRequest(PassphraseRequest {
						message: resp.take_message()?,
						client: self,
						result_handler: result_handler,
					}))
				}
				mtype => Err(Error::UnexpectedMessageType(mtype)),
			}
		}
	}

	pub fn init_device(&mut self) -> Result<()> {
		self.features = Some(self.initialize()?.ok()?);
		Ok(())
	}

	//TODO(stevenroose) macronize all the things!

	pub fn initialize(&mut self) -> Result<TrezorResponse<Features, Features>> {
		let mut req = protos::Initialize::new();
		req.set_state(Vec::new());
		self.call(req, Box::new(|m| Ok(m)))
	}

	pub fn ping(&mut self, message: &str) -> Result<TrezorResponse<(), Success>> {
		let mut req = protos::Ping::new();
		req.set_message(message.to_owned());
		self.call(req, Box::new(|_| Ok(())))
	}

	pub fn change_pin(&mut self, remove: bool) -> Result<TrezorResponse<(), Success>> {
		let mut req = protos::ChangePin::new();
		req.set_remove(remove);
		self.call(req, Box::new(|_| Ok(())))
	}

	pub fn wipe_device(&mut self) -> Result<TrezorResponse<(), Success>> {
		let req = protos::WipeDevice::new();
		self.call(req, Box::new(|_| Ok(())))
	}

	pub fn recover_device(
		&mut self,
		word_count: WordCount,
		passphrase_protection: bool,
		pin_protection: bool,
		label: String,
		dry_run: bool,
	) -> Result<TrezorResponse<(), Success>> {
		let mut req = protos::RecoveryDevice::new();
		req.set_word_count(word_count as u32);
		req.set_passphrase_protection(passphrase_protection);
		req.set_pin_protection(pin_protection);
		req.set_label(label);
		req.set_enforce_wordlist(true);
		req.set_dry_run(dry_run);
		req.set_field_type(
			protos::RecoveryDevice_RecoveryDeviceType::RecoveryDeviceType_ScrambledWords,
		);
		//TODO(stevenroose) support languages
		req.set_language("english".to_owned());
		self.call(req, Box::new(|_| Ok(())))
	}

	pub fn reset_device(
		&mut self,
		display_random: bool,
		strength: usize,
		passphrase_protection: bool,
		pin_protection: bool,
		label: String,
		skip_backup: bool,
		no_backup: bool,
	) -> Result<TrezorResponse<(), Success>> {
		let mut req = protos::ResetDevice::new();
		req.set_display_random(display_random);
		req.set_strength(strength as u32);
		req.set_passphrase_protection(passphrase_protection);
		req.set_pin_protection(pin_protection);
		req.set_label(label);
		req.set_skip_backup(skip_backup);
		req.set_no_backup(no_backup);
		//TODO(stevenroose) support languages
		req.set_language("english".to_owned());
		self.call(req, Box::new(|_| Ok(())))
	}

	pub fn backup(&mut self) -> Result<TrezorResponse<(), Success>> {
		let req = protos::BackupDevice::new();
		self.call(req, Box::new(|_| Ok(())))
	}

	//TODO(stevenroose) support U2F stuff? currently ignored all

	pub fn apply_settings(
		&mut self,
		label: Option<String>,
		use_passphrase: Option<bool>,
		homescreen: Option<Vec<u8>>,
		passphrase_source: Option<PassphraseSource>,
		auto_lock_delay_ms: Option<usize>,
	) -> Result<TrezorResponse<(), Success>> {
		let mut req = protos::ApplySettings::new();
		if let Some(label) = label {
			req.set_label(label);
		}
		if let Some(use_passphrase) = use_passphrase {
			req.set_use_passphrase(use_passphrase);
		}
		if let Some(homescreen) = homescreen {
			req.set_homescreen(homescreen);
		}
		if let Some(passphrase_source) = passphrase_source {
			req.set_passphrase_source(passphrase_source);
		}
		if let Some(auto_lock_delay_ms) = auto_lock_delay_ms {
			req.set_auto_lock_delay_ms(auto_lock_delay_ms as u32);
		}
		self.call(req, Box::new(|_| Ok(())))
	}

	pub fn get_public_key(
		&mut self,
		path: Vec<u32>,
		show_display: bool,
		script_type: InputScriptType,
	) -> Result<TrezorResponse<PublicKey, PublicKey>> {
		let mut req = protos::GetPublicKey::new();
		req.set_address_n(path);
		req.set_show_display(show_display);
		req.set_script_type(script_type);
		self.call(req, Box::new(|m| Ok(m)))
	}
}
