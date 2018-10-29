use bitcoin::network::constants::Network; //TODO(stevenroose) change after https://github.com/rust-bitcoin/rust-bitcoin/pull/181
use bitcoin::util::hash::Sha256dHash;
use bitcoin::util::psbt;
use secp256k1;

use error::{Error, Result};

/// Find the (first if multiple) PSBT input that refers to the given txid.
pub fn psbt_find_input(
	psbt: &psbt::PartiallySignedTransaction,
	txid: Sha256dHash,
) -> Result<&psbt::Input> {
	let inputs = &psbt.global.unsigned_tx.input;
	let opt = inputs.iter().enumerate().find(|i| i.1.previous_output.txid == txid);
	let idx = opt.ok_or(Error::TxRequestUnknownTxid(txid))?.0;
	psbt.inputs.get(idx).ok_or(Error::TxRequestInvalidIndex(idx))
}

/// Get a hash from a reverse byte representation.
pub fn from_rev_bytes(rev_bytes: &[u8]) -> Sha256dHash {
	let mut bytes = rev_bytes.to_vec();
	bytes.reverse();
	bytes.as_slice().into()
}

/// Get the reverse byte representation of a hash.
pub fn to_rev_bytes(hash: &Sha256dHash) -> [u8; 32] {
	let mut bytes = hash.to_bytes();
	bytes.reverse();
	bytes
}

/// Parse a Bitcoin Core-style 65-byte recoverable signature.
//TODO(stevenroose) potentially replace this with native method if it gets merged:
// https://github.com/rust-bitcoin/rust-secp256k1/pull/74
pub fn parse_recoverable_signature(sig: &[u8]) -> Result<secp256k1::RecoverableSignature> {
	if sig.len() != 65 {
		return Err(secp256k1::Error::InvalidSignature.into());
	}

	// Bitcoin Core sets the first byte to `27 + rec + (fCompressed ? 4 : 0)`.
	let rec_id = secp256k1::RecoveryId::from_i32(if sig[0] >= 31 {
		(sig[0] - 31) as i32
	} else {
		(sig[0] - 27) as i32
	})?;

	let secp = secp256k1::Secp256k1::without_caps();
	Ok(secp256k1::RecoverableSignature::from_compact(&secp, &sig[1..], rec_id)?)
}

/// Convert a bitcoin network constant to the Trezor-compatible coin_name string.
pub fn coin_name(network: Network) -> Result<String> {
	match network {
		Network::Bitcoin => Ok("Bitcoin".to_owned()),
		Network::Testnet => Ok("Testnet".to_owned()),
		_ => Err(Error::UnsupportedNetwork),
	}
}
