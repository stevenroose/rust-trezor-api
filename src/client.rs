use std::fmt;

use bitcoin::network::constants::Network; //TODO(stevenroose) change after https://github.com/rust-bitcoin/rust-bitcoin/pull/181
use bitcoin::util::bip32;
use bitcoin::util::hash::Sha256dHash;
use bitcoin::util::psbt;
use bitcoin::{Address, Transaction};
use hex;
use secp256k1;
use unicode_normalization::UnicodeNormalization;

use super::Model;
use error::{Error, Result};
use messages::TrezorMessage;
use protos;
use protos::MessageType::*;
use transport::{ProtoMessage, Transport};
use utils;

// Some types with raw protos that we use in the public interface so they have to be exported.
use protos::ApplySettings_PassphraseSourceType as PassphraseSource;
pub use protos::ButtonRequest_ButtonRequestType as ButtonRequestType;
pub use protos::Features;
pub use protos::InputScriptType;
pub use protos::PinMatrixRequest_PinMatrixRequestType as PinMatrixRequestType;
use protos::TxAck_TransactionType_TxOutputType_OutputScriptType as OutputScriptType;
use protos::TxRequest_RequestType as TxRequestType;

/// The different options for the number of words in a seed phrase.
pub enum WordCount {
	W12 = 12,
	W18 = 18,
	W24 = 24,
}

/// The different types of user interactions the Trezor device can request.
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum InteractionType {
	Button,
	PinMatrix,
	Passphrase,
}

//TODO(stevenroose) should this be FnOnce and put in an FnBox?
/// Function to be passed to the `Trezor.call` method to process the Trezor response message into a
/// general-purpose type.
pub type ResultHandler<'a, T, R> = Fn(&'a mut Trezor, R) -> Result<T>;

/// A button request message sent by the device.
pub struct ButtonRequest<'a, T, R: TrezorMessage> {
	message: protos::ButtonRequest,
	client: &'a mut Trezor,
	result_handler: Box<ResultHandler<'a, T, R>>,
}

impl<'a, T, R: TrezorMessage> fmt::Debug for ButtonRequest<'a, T, R> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		fmt::Debug::fmt(&self.message, f)
	}
}

impl<'a, T, R: TrezorMessage> ButtonRequest<'a, T, R> {
	/// The type of button request.
	pub fn request_type(&self) -> ButtonRequestType {
		self.message.get_code()
	}

	/// The metadata sent with the button request.
	pub fn request_data(&self) -> &str {
		self.message.get_data()
	}

	/// Ack the request and get the next message from the device.
	pub fn ack(self) -> Result<TrezorResponse<'a, T, R>> {
		let req = protos::ButtonAck::new();
		self.client.call(req, self.result_handler)
	}
}

/// A PIN matrix request message sent by the device.
pub struct PinMatrixRequest<'a, T, R: TrezorMessage> {
	message: protos::PinMatrixRequest,
	client: &'a mut Trezor,
	result_handler: Box<ResultHandler<'a, T, R>>,
}

impl<'a, T, R: TrezorMessage> fmt::Debug for PinMatrixRequest<'a, T, R> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		fmt::Debug::fmt(&self.message, f)
	}
}

impl<'a, T, R: TrezorMessage> PinMatrixRequest<'a, T, R> {
	/// The type of PIN matrix request.
	pub fn request_type(&self) -> PinMatrixRequestType {
		self.message.get_field_type()
	}

	/// Ack the request with a PIN and get the next message from the device.
	pub fn ack_pin(self, pin: String) -> Result<TrezorResponse<'a, T, R>> {
		let mut req = protos::PinMatrixAck::new();
		req.set_pin(pin);
		self.client.call(req, self.result_handler)
	}
}

/// A passphrase request message sent by the device.
pub struct PassphraseRequest<'a, T, R: TrezorMessage> {
	message: protos::PassphraseRequest,
	client: &'a mut Trezor,
	result_handler: Box<ResultHandler<'a, T, R>>,
}

impl<'a, T, R: TrezorMessage> fmt::Debug for PassphraseRequest<'a, T, R> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		fmt::Debug::fmt(&self.message, f)
	}
}

impl<'a, T, R: TrezorMessage> PassphraseRequest<'a, T, R> {
	/// Check whether the use is supposed to enter the passphrase on the device or not.
	pub fn on_device(&self) -> bool {
		self.message.get_on_device()
	}

	/// Ack the request with a passphrase and get the next message from the device.
	pub fn ack_passphrase(self, passphrase: String) -> Result<TrezorResponse<'a, T, R>> {
		let mut req = protos::PassphraseAck::new();
		req.set_passphrase(passphrase);
		self.client.call(req, self.result_handler)
	}

	/// Ack the request without a passphrase to let the user enter it on the device
	/// and get the next message from the device.
	pub fn ack(self) -> Result<TrezorResponse<'a, T, R>> {
		let req = protos::PassphraseAck::new();
		self.client.call(req, self.result_handler)
	}
}

/// A response from a Trezor device.  On every message exchange, instead of the expected/desired
/// response, the Trezor can ask for some user interaction, or can send a failure.
#[derive(Debug)]
pub enum TrezorResponse<'a, T, R: TrezorMessage> {
	Ok(T),
	Failure(protos::Failure),
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
	/// Get the actual `Ok` response value or an error if not `Ok`.
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

	/// Get the button request object or an error if not `ButtonRequest`.
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

	/// Get the PIN matrix request object or an error if not `PinMatrixRequest`.
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

	/// Get the passphrase request object or an error if not `PassphraseRequest`.
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

/// When resetting the device, it will ask for entropy to aid key generation.
pub struct EntropyRequest<'a> {
	client: &'a mut Trezor,
}

impl<'a> EntropyRequest<'a> {
	/// Provide exactly 32 bytes or entropy.
	pub fn ack_entropy(self, entropy: Vec<u8>) -> Result<TrezorResponse<'a, (), protos::Success>> {
		if entropy.len() != 32 {
			return Err(Error::InvalidEntropy);
		}

		let mut req = protos::EntropyAck::new();
		req.set_entropy(entropy);
		self.client.call(req, Box::new(|_, _| Ok(())))
	}
}

/// Fulfill a TxRequest for TXINPUT.
fn ack_input_request(
	req: &protos::TxRequest,
	psbt: &psbt::PartiallySignedTransaction,
) -> Result<protos::TxAck> {
	if !req.has_details() || !req.get_details().has_request_index() {
		return Err(Error::MalformedTxRequest(req.clone()));
	}

	// Choose either the tx we are signing or a dependent tx.
	let input_index = req.get_details().get_request_index() as usize;
	let input = if req.get_details().has_tx_hash() {
		let req_hash: Sha256dHash = utils::from_rev_bytes(req.get_details().get_tx_hash());
		trace!("Preparing ack for input {}:{}", req_hash, input_index);
		let inp = utils::psbt_find_input(&psbt, req_hash)?;
		let tx = inp.non_witness_utxo.as_ref().ok_or(Error::PsbtMissingInputTx(req_hash))?;
		let opt = &tx.input.get(input_index);
		opt.ok_or(Error::TxRequestInvalidIndex(input_index))?
	} else {
		trace!("Preparing ack for tx input #{}", input_index);
		let opt = &psbt.global.unsigned_tx.input.get(input_index);
		opt.ok_or(Error::TxRequestInvalidIndex(input_index))?
	};

	let mut data_input = protos::TxAck_TransactionType_TxInputType::new();
	data_input.set_prev_hash(utils::to_rev_bytes(&input.previous_output.txid).to_vec());
	data_input.set_prev_index(input.previous_output.vout);
	data_input.set_script_sig(input.script_sig.to_bytes());
	data_input.set_sequence(input.sequence);

	// Extra data only for currently signing tx.
	if !req.get_details().has_tx_hash() {
		let psbt_input = psbt
			.inputs
			.get(input_index)
			.ok_or(Error::InvalidPsbt("not enough psbt inputs".to_owned()))?;

		// If there is exactly 1 HD keypath known, we can provide it.  If more it's multisig.
		if psbt_input.hd_keypaths.len() == 1 {
			data_input.set_address_n(
				(psbt_input.hd_keypaths.iter().nth(0).unwrap().1)
					.1
					.iter()
					.map(|i| i.clone().into())
					.collect(),
			);
		}
		//TODO(stevenroose) script_type
		//TODO(stevenroose) multisig

		data_input.set_amount(if let Some(utxo) = &psbt_input.witness_utxo {
			utxo.value
		} else if let Some(ref tx) = psbt_input.non_witness_utxo {
			tx.output
				.get(input.previous_output.vout as usize)
				.ok_or(Error::InvalidPsbt("utxo tx output length mismatch".to_owned()))?
				.value
		} else {
			return Err(Error::InvalidPsbt(format!("no utxo for PSBT input {}", input_index)));
		});
	}

	trace!("Prepared input to ack: {:?}", data_input);
	let mut txdata = protos::TxAck_TransactionType::new();
	txdata.mut_inputs().push(data_input);
	let mut msg = protos::TxAck::new();
	msg.set_tx(txdata);
	Ok(msg)
}

/// Fulfill a TxRequest for TXOUTPUT.
fn ack_output_request(
	req: &protos::TxRequest,
	psbt: &psbt::PartiallySignedTransaction,
	network: Network,
) -> Result<protos::TxAck> {
	if !req.has_details() || !req.get_details().has_request_index() {
		return Err(Error::MalformedTxRequest(req.clone()));
	}

	// For outputs, the Trezor only needs bin_outputs to be set for dependent txs and full outputs
	// for the signing tx.
	let mut txdata = protos::TxAck_TransactionType::new();
	if req.get_details().has_tx_hash() {
		// Dependent tx, take the output from the PSBT and just create bin_output.
		let output_index = req.get_details().get_request_index() as usize;
		let req_hash: Sha256dHash = utils::from_rev_bytes(req.get_details().get_tx_hash());
		trace!("Preparing ack for output {}:{}", req_hash, output_index);
		let inp = utils::psbt_find_input(&psbt, req_hash)?;
		let output = if let Some(ref tx) = inp.non_witness_utxo {
			let opt = &tx.output.get(output_index);
			opt.ok_or(Error::TxRequestInvalidIndex(output_index))?
		} else if let Some(ref utxo) = inp.witness_utxo {
			utxo
		} else {
			return Err(Error::InvalidPsbt("not all inputs have utxo data".to_owned()));
		};

		let mut bin_output = protos::TxAck_TransactionType_TxOutputBinType::new();
		bin_output.set_amount(output.value);
		bin_output.set_script_pubkey(output.script_pubkey.to_bytes());

		trace!("Prepared bin_output to ack: {:?}", bin_output);
		txdata.mut_bin_outputs().push(bin_output);
	} else {
		// Signing tx, we need to fill the full output meta object.
		let output_index = req.get_details().get_request_index() as usize;
		trace!("Preparing ack for tx output #{}", output_index);
		let opt = &psbt.global.unsigned_tx.output.get(output_index);
		let output = opt.ok_or(Error::TxRequestInvalidIndex(output_index))?;

		let mut data_output = protos::TxAck_TransactionType_TxOutputType::new();
		data_output.set_amount(output.value);
		// Set script type to PAYTOADDRESS unless we find out otherwise from the PSBT.
		data_output.set_script_type(OutputScriptType::PAYTOADDRESS);
		if let Some(addr) = utils::address_from_script(&output.script_pubkey, network) {
			data_output.set_address(addr.to_string());
		}

		let psbt_output = psbt
			.outputs
			.get(output_index)
			.ok_or(Error::InvalidPsbt("output indices don't match".to_owned()))?;
		if psbt_output.hd_keypaths.len() == 1 {
			data_output.set_address_n(
				(psbt_output.hd_keypaths.iter().nth(0).unwrap().1)
					.1
					.iter()
					.map(|i| i.clone().into())
					.collect(),
			);

			// Since we know the keypath, it's probably a change output.  So update script_type.
			let script_pubkey = &psbt.global.unsigned_tx.output[output_index].script_pubkey;
			if script_pubkey.is_op_return() {
				data_output.set_script_type(OutputScriptType::PAYTOOPRETURN);
				data_output.set_op_return_data(script_pubkey.as_bytes()[1..].to_vec());
			} else if psbt_output.witness_script.is_some() {
				if psbt_output.redeem_script.is_some() {
					data_output.set_script_type(OutputScriptType::PAYTOP2SHWITNESS);
				} else {
					data_output.set_script_type(OutputScriptType::PAYTOWITNESS);
				}
			} else {
				data_output.set_script_type(OutputScriptType::PAYTOADDRESS);
			}
		}

		trace!("Prepared output to ack: {:?}", data_output);
		txdata.mut_outputs().push(data_output);
	};

	let mut msg = protos::TxAck::new();
	msg.set_tx(txdata);
	Ok(msg)
}

/// Fulfill a TxRequest for TXMETA.
fn ack_meta_request(
	req: &protos::TxRequest,
	psbt: &psbt::PartiallySignedTransaction,
) -> Result<protos::TxAck> {
	if !req.has_details() {
		return Err(Error::MalformedTxRequest(req.clone()));
	}

	// Choose either the tx we are signing or a dependent tx.
	let tx: &Transaction = if req.get_details().has_tx_hash() {
		// dependeny tx, look for it in PSBT inputs
		let req_hash: Sha256dHash = utils::from_rev_bytes(req.get_details().get_tx_hash());
		trace!("Preparing ack for tx meta of {}", req_hash);
		let inp = utils::psbt_find_input(&psbt, req_hash)?;
		inp.non_witness_utxo.as_ref().ok_or(Error::PsbtMissingInputTx(req_hash))?
	} else {
		// currently signing tx
		trace!("Preparing ack for tx meta of tx being signed");
		&psbt.global.unsigned_tx
	};

	let mut txdata = protos::TxAck_TransactionType::new();
	txdata.set_version(tx.version);
	txdata.set_lock_time(tx.lock_time);
	txdata.set_inputs_cnt(tx.input.len() as u32);
	txdata.set_outputs_cnt(tx.output.len() as u32);
	//TODO(stevenroose) python does something with extra data?

	trace!("Prepared tx meta to ack: {:?}", txdata);
	let mut msg = protos::TxAck::new();
	msg.set_tx(txdata);
	Ok(msg)
}

/// Object to track the progress in the transaction signing flow.  The device will ask for various
/// parts of the transaction and dependent transactions and can at any point also ask for user
/// interaction.  The information asked for by the device is provided based on a PSBT object and the
/// resulting extra signatures are also added to the PSBT file.
///
/// It's important to always first call the `apply()` method with the PSBT object to update with the
/// data from the device and only when it returns false (not finished), call the `ack_psbt()` method
/// to send more information to the device.
pub struct SignTxProgress<'a> {
	client: &'a mut Trezor,
	req: protos::TxRequest,
}

impl<'a> SignTxProgress<'a> {
	/// Inspector to the request message received from the device.
	pub fn tx_request(&self) -> &protos::TxRequest {
		&self.req
	}

	/// Check whether or not the signing process is finished.
	pub fn finished(&self) -> bool {
		self.req.get_request_type() == TxRequestType::TXFINISHED
	}

	/// Applies the updates received from the device to the PSBT and returns whether or not
	/// the signing process is finished.
	pub fn apply(&self, psbt: &mut psbt::PartiallySignedTransaction) -> Result<bool> {
		if self.req.has_serialized() {
			let serialized = self.req.get_serialized();
			if serialized.has_signature_index() {
				let sig_idx = serialized.get_signature_index() as usize;
				let sig_bytes = serialized.get_signature();
				if sig_idx >= psbt.inputs.len() {
					return Err(Error::TxRequestInvalidIndex(sig_idx));
				}
				trace!("Adding signature #{}: {}", sig_idx, hex::encode(sig_bytes));
				psbt.inputs[sig_idx].final_script_sig = Some(sig_bytes.to_vec().into());
			}
			//TODO(stevenroose) handle serialized_tx if we need this
		}

		Ok(self.finished())
	}

	/// Manually provide a TxAck message to the device.
	///
	/// This method will panic if `finished()` or `apply()` returned true,
	/// so it should always be checked in advance.
	pub fn ack_msg(
		self,
		ack: protos::TxAck,
	) -> Result<TrezorResponse<'a, SignTxProgress<'a>, protos::TxRequest>> {
		assert!(!self.finished());

		self.client.call(
			ack,
			Box::new(|c, m| {
				Ok(SignTxProgress {
					req: m,
					client: c,
				})
			}),
		)
	}

	/// Provide additional PSBT information to the device.
	///
	/// This method will panic if `apply()` returned true,
	/// so it should always be checked in advance.
	pub fn ack_psbt(
		self,
		psbt: &psbt::PartiallySignedTransaction,
		network: Network,
	) -> Result<TrezorResponse<'a, SignTxProgress<'a>, protos::TxRequest>> {
		assert!(self.req.get_request_type() != TxRequestType::TXFINISHED);

		let ack = match self.req.get_request_type() {
			TxRequestType::TXINPUT => ack_input_request(&self.req, &psbt),
			TxRequestType::TXOUTPUT => ack_output_request(&self.req, &psbt, network),
			TxRequestType::TXMETA => ack_meta_request(&self.req, &psbt),
			TxRequestType::TXEXTRADATA => unimplemented!(), //TODO(stevenroose) implement
			TxRequestType::TXFINISHED => unreachable!(),
		}?;
		self.ack_msg(ack)
	}
}

/// A Trezor client.
pub struct Trezor {
	pub model: Model,
	// Cached features for later inspection.
	pub features: Option<protos::Features>,
	transport: Box<Transport>,
}

/// Create a new Trezor instance with the given transport.
pub fn trezor_with_transport(model: Model, transport: Box<Transport>) -> Trezor {
	Trezor {
		model: model,
		transport: transport,
		features: None,
	}
}

impl Trezor {
	/// Sends a message and returns the raw ProtoMessage struct that was responded by the device.
	/// This method is only exported for users that want to expand the features of this library
	/// f.e. for supporting additional coins etc.
	pub fn call_raw<S: TrezorMessage>(&mut self, message: S) -> Result<ProtoMessage> {
		let proto_msg = ProtoMessage(S::message_type(), message.write_to_bytes()?);
		self.transport.write_message(proto_msg).map_err(|e| Error::TransportSendMessage(e))?;
		self.transport.read_message().map_err(|e| Error::TransportReceiveMessage(e))
	}

	/// Sends a message and returns a TrezorResponse with either the expected response message,
	/// a failure or an interaction request.
	/// This method is only exported for users that want to expand the features of this library
	/// f.e. for supporting additional coins etc.
	pub fn call<'a, T, S: TrezorMessage, R: TrezorMessage>(
		&'a mut self,
		message: S,
		result_handler: Box<ResultHandler<'a, T, R>>,
	) -> Result<TrezorResponse<'a, T, R>> {
		trace!("Sending {:?} msg: {:?}", S::message_type(), message);
		let resp = self.call_raw(message)?;
		if resp.message_type() == R::message_type() {
			let resp_msg = resp.take_message()?;
			trace!("Received {:?} msg: {:?}", R::message_type(), resp_msg);
			Ok(TrezorResponse::Ok(result_handler(self, resp_msg)?))
		} else {
			match resp.message_type() {
				MessageType_Failure => {
					let fail_msg = resp.take_message()?;
					debug!("Received failure: {:?}", fail_msg);
					Ok(TrezorResponse::Failure(fail_msg))
				}
				MessageType_ButtonRequest => {
					let req_msg = resp.take_message()?;
					trace!("Received ButtonRequest: {:?}", req_msg);
					Ok(TrezorResponse::ButtonRequest(ButtonRequest {
						message: req_msg,
						client: self,
						result_handler: result_handler,
					}))
				}
				MessageType_PinMatrixRequest => {
					let req_msg = resp.take_message()?;
					trace!("Received PinMatrixRequest: {:?}", req_msg);
					Ok(TrezorResponse::PinMatrixRequest(PinMatrixRequest {
						message: req_msg,
						client: self,
						result_handler: result_handler,
					}))
				}
				MessageType_PassphraseRequest => {
					let req_msg = resp.take_message()?;
					trace!("Received PassphraseRequest: {:?}", req_msg);
					Ok(TrezorResponse::PassphraseRequest(PassphraseRequest {
						message: req_msg,
						client: self,
						result_handler: result_handler,
					}))
				}
				mtype => {
					debug!(
						"Received unexpected msg type: {:?}; raw msg: {}",
						mtype,
						hex::encode(resp.take_payload())
					);
					Err(Error::UnexpectedMessageType(mtype))
				}
			}
		}
	}

	pub fn init_device(&mut self) -> Result<()> {
		let features = self.initialize()?.ok()?;
		self.features = Some(features);
		Ok(())
	}

	//TODO(stevenroose) macronize all the things!

	pub fn initialize(&mut self) -> Result<TrezorResponse<Features, Features>> {
		let mut req = protos::Initialize::new();
		req.set_state(Vec::new());
		self.call(req, Box::new(|_, m| Ok(m)))
	}

	pub fn ping(&mut self, message: &str) -> Result<TrezorResponse<(), protos::Success>> {
		let mut req = protos::Ping::new();
		req.set_message(message.to_owned());
		self.call(req, Box::new(|_, _| Ok(())))
	}

	pub fn change_pin(&mut self, remove: bool) -> Result<TrezorResponse<(), protos::Success>> {
		let mut req = protos::ChangePin::new();
		req.set_remove(remove);
		self.call(req, Box::new(|_, _| Ok(())))
	}

	pub fn wipe_device(&mut self) -> Result<TrezorResponse<(), protos::Success>> {
		let req = protos::WipeDevice::new();
		self.call(req, Box::new(|_, _| Ok(())))
	}

	pub fn recover_device(
		&mut self,
		word_count: WordCount,
		passphrase_protection: bool,
		pin_protection: bool,
		label: String,
		dry_run: bool,
	) -> Result<TrezorResponse<(), protos::Success>> {
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
		self.call(req, Box::new(|_, _| Ok(())))
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
	) -> Result<TrezorResponse<EntropyRequest, protos::EntropyRequest>> {
		let mut req = protos::ResetDevice::new();
		req.set_display_random(display_random);
		req.set_strength(strength as u32);
		req.set_passphrase_protection(passphrase_protection);
		req.set_pin_protection(pin_protection);
		req.set_label(label);
		req.set_skip_backup(skip_backup);
		req.set_no_backup(no_backup);
		self.call(
			req,
			Box::new(|c, _| {
				Ok(EntropyRequest {
					client: c,
				})
			}),
		)
	}

	pub fn backup(&mut self) -> Result<TrezorResponse<(), protos::Success>> {
		let req = protos::BackupDevice::new();
		self.call(req, Box::new(|_, _| Ok(())))
	}

	//TODO(stevenroose) support U2F stuff? currently ignored all

	pub fn apply_settings(
		&mut self,
		label: Option<String>,
		use_passphrase: Option<bool>,
		homescreen: Option<Vec<u8>>,
		passphrase_source: Option<PassphraseSource>,
		auto_lock_delay_ms: Option<usize>,
	) -> Result<TrezorResponse<(), protos::Success>> {
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
		self.call(req, Box::new(|_, _| Ok(())))
	}

	pub fn get_public_key(
		&mut self,
		path: Vec<bip32::ChildNumber>,
		script_type: InputScriptType,
		network: Network,
		show_display: bool,
	) -> Result<TrezorResponse<bip32::ExtendedPubKey, protos::PublicKey>> {
		let mut req = protos::GetPublicKey::new();
		req.set_address_n(path.into_iter().map(Into::into).collect());
		req.set_show_display(show_display);
		req.set_coin_name(utils::coin_name(network)?);
		req.set_script_type(script_type);
		self.call(req, Box::new(|_, m| Ok(m.get_xpub().parse()?)))
	}

	//TODO(stevenroose) multisig
	pub fn get_address(
		&mut self,
		path: Vec<bip32::ChildNumber>,
		script_type: InputScriptType,
		network: Network,
		show_display: bool,
	) -> Result<TrezorResponse<Address, protos::Address>> {
		let mut req = protos::GetAddress::new();
		req.set_address_n(path.into_iter().map(Into::into).collect());
		req.set_coin_name(utils::coin_name(network)?);
		req.set_show_display(show_display);
		req.set_script_type(script_type);
		self.call(req, Box::new(|_, m| Ok(m.get_address().parse()?)))
	}

	pub fn sign_tx(
		&mut self,
		psbt: &psbt::PartiallySignedTransaction,
		network: Network,
	) -> Result<TrezorResponse<SignTxProgress, protos::TxRequest>> {
		let tx = &psbt.global.unsigned_tx;
		let mut req = protos::SignTx::new();
		req.set_inputs_count(tx.input.len() as u32);
		req.set_outputs_count(tx.output.len() as u32);
		req.set_coin_name(utils::coin_name(network)?);
		req.set_version(tx.version);
		req.set_lock_time(tx.lock_time);
		self.call(
			req,
			Box::new(|c, m| {
				Ok(SignTxProgress {
					req: m,
					client: c,
				})
			}),
		)
	}

	pub fn sign_message(
		&mut self,
		message: String,
		path: Vec<bip32::ChildNumber>,
		script_type: InputScriptType,
		network: Network,
	) -> Result<TrezorResponse<(Address, secp256k1::RecoverableSignature), protos::MessageSignature>>
	{
		let mut req = protos::SignMessage::new();
		req.set_address_n(path.into_iter().map(Into::into).collect());
		// Normalize to Unicode NFC.
		let msg_bytes = message.nfc().collect::<String>().into_bytes();
		req.set_message(msg_bytes);
		req.set_coin_name(utils::coin_name(network)?);
		req.set_script_type(script_type);
		self.call(
			req,
			Box::new(|_, m| {
				let address = m.get_address().parse()?;
				let signature = utils::parse_recoverable_signature(m.get_signature())?;
				Ok((address, signature))
			}),
		)
	}
}
