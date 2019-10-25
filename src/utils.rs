use bitcoin::blockdata::script::Script;
use bitcoin::network::constants::Network; //TODO(stevenroose) change after https://github.com/rust-bitcoin/rust-bitcoin/pull/181
use bitcoin::util::{address, bip32, psbt};
use bitcoin_bech32::{u5, WitnessProgram};
use bitcoin_hashes::{hash160, sha256d, Hash};
use secp256k1;

use crate::error::{Error, Result};

/// convert Network to bech32 network (this should go away soon)
fn bech_network(network: Network) -> bitcoin_bech32::constants::Network {
	match network {
		Network::Bitcoin => bitcoin_bech32::constants::Network::Bitcoin,
		Network::Testnet => bitcoin_bech32::constants::Network::Testnet,
		Network::Regtest => bitcoin_bech32::constants::Network::Regtest,
	}
}

/// Retrieve an address from the given script.
pub fn address_from_script(script: &Script, network: Network) -> Option<address::Address> {
	Some(address::Address {
		payload: if script.is_p2sh() {
			address::Payload::ScriptHash(hash160::Hash::from_slice(&script[2..22]).unwrap())
		} else if script.is_p2pkh() {
			address::Payload::PubkeyHash(hash160::Hash::from_slice(&script[3..23]).unwrap())
		} else if script.is_v0_p2wsh() {
			match WitnessProgram::new(
				u5::try_from_u8(0).expect("0<32"),
				script.as_bytes()[2..34].to_vec(),
				bech_network(network),
			) {
				Ok(prog) => address::Payload::WitnessProgram(prog),
				Err(_) => return None,
			}
		} else if script.is_v0_p2wpkh() {
			match WitnessProgram::new(
				u5::try_from_u8(0).expect("0<32"),
				script.as_bytes()[2..22].to_vec(),
				bech_network(network),
			) {
				Ok(prog) => address::Payload::WitnessProgram(prog),
				Err(_) => return None,
			}
		} else {
			return None;
		},
		network: network,
	})
}

/// Find the (first if multiple) PSBT input that refers to the given txid.
pub fn psbt_find_input(
	psbt: &psbt::PartiallySignedTransaction,
	txid: sha256d::Hash,
) -> Result<&psbt::Input> {
	let inputs = &psbt.global.unsigned_tx.input;
	let opt = inputs.iter().enumerate().find(|i| i.1.previous_output.txid == txid);
	let idx = opt.ok_or(Error::TxRequestUnknownTxid(txid))?.0;
	psbt.inputs.get(idx).ok_or(Error::TxRequestInvalidIndex(idx))
}

/// Get a hash from a reverse byte representation.
pub fn from_rev_bytes(rev_bytes: &[u8]) -> Option<sha256d::Hash> {
	let mut bytes = rev_bytes.to_vec();
	bytes.reverse();
	sha256d::Hash::from_slice(&bytes).ok()
}

/// Get the reverse byte representation of a hash.
pub fn to_rev_bytes(hash: &sha256d::Hash) -> [u8; 32] {
	let mut bytes = hash.clone().into_inner();
	bytes.reverse();
	bytes
}

/// Parse a Bitcoin Core-style 65-byte recoverable signature.
pub fn parse_recoverable_signature(
	sig: &[u8],
) -> std::result::Result<secp256k1::RecoverableSignature, secp256k1::Error> {
	if sig.len() != 65 {
		return Err(secp256k1::Error::InvalidSignature);
	}

	// Bitcoin Core sets the first byte to `27 + rec + (fCompressed ? 4 : 0)`.
	let rec_id = secp256k1::RecoveryId::from_i32(if sig[0] >= 31 {
		(sig[0] - 31) as i32
	} else {
		(sig[0] - 27) as i32
	})?;

	Ok(secp256k1::RecoverableSignature::from_compact(&sig[1..], rec_id)?)
}

/// Convert a bitcoin network constant to the Trezor-compatible coin_name string.
pub fn coin_name(network: Network) -> Result<String> {
	match network {
		Network::Bitcoin => Ok("Bitcoin".to_owned()),
		Network::Testnet => Ok("Testnet".to_owned()),
		_ => Err(Error::UnsupportedNetwork),
	}
}

/// Convert a BIP-32 derivation path into a Vec<u32>.
pub fn convert_path(path: &bip32::DerivationPath) -> Vec<u32> {
	path.into_iter().map(|i| u32::from(*i)).collect()
}
