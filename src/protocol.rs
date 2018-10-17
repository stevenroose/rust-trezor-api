use std::cmp;

use byteorder::{BigEndian, ByteOrder};

use error::{Error, Result};
use protos::MessageType;

pub trait Transport {
	fn write_chunk(&mut self, chunk: Vec<u8>) -> Result<()>;
	fn read_chunk(&mut self) -> Result<Vec<u8>>;
}

pub trait Protocol {
	fn session_begin(&mut self) -> Result<()>;
	fn session_end(&mut self) -> Result<()>;
	fn write(&mut self, message_type: MessageType, payload: Vec<u8>) -> Result<()>;
	fn read(&mut self, message_type: MessageType) -> Result<Vec<u8>>;
}

const REPLEN: usize = 64;

pub struct ProtocolV2<T: Transport> {
	pub transport: T,
	pub session_id: u32,
}

impl<T: Transport> Protocol for ProtocolV2<T> {
	fn session_begin(&mut self) -> Result<()> {
		let mut chunk = vec![0; REPLEN];
		chunk[0] = 0x03;
		self.transport.write_chunk(chunk)?;
		let resp = self.transport.read_chunk()?;
		if resp[0] != 0x03 {
			return Err(Error::DeviceBadMagic);
		}
		self.session_id = BigEndian::read_u32(&resp[1..5]);
		Ok(())
	}

	fn session_end(&mut self) -> Result<()> {
		assert!(self.session_id != 0);
		let mut chunk = vec![0; REPLEN];
		chunk[0] = 0x04;
		BigEndian::write_u32(&mut chunk[1..5], self.session_id);
		self.transport.write_chunk(chunk)?;
		let resp = self.transport.read_chunk()?;
		if resp[0] != 0x04 {
			return Err(Error::DeviceBadMagic);
		}
		self.session_id = 0;
		Ok(())
	}

	fn write(&mut self, message_type: MessageType, payload: Vec<u8>) -> Result<()> {
		assert!(self.session_id != 0);

		// First generate the total payload, then write it to the transport in chunks.
		let mut data = Vec::with_capacity(8);
		BigEndian::write_u32(&mut data[0..4], message_type as u32);
		BigEndian::write_u32(&mut data[4..8], payload.len() as u32);
		data.extend(payload);

		let mut cur: usize = 0;
		let mut seq: isize = -1;
		while cur < data.len() {
			// Build header.
			let mut chunk = if seq < 0 {
				let mut header = Vec::with_capacity(5);
				header[0] = 0x01;
				BigEndian::write_u32(&mut header[1..5], self.session_id);
				header
			} else {
				let mut header = Vec::with_capacity(9);
				header[0] = 0x01;
				BigEndian::write_u32(&mut header[1..5], self.session_id);
				BigEndian::write_u32(&mut header[5..9], seq as u32);
				header
			};
			seq += 1;

			// Fill remainder.
			let end = cmp::min(cur + (REPLEN - chunk.len()), data.len());
			chunk.extend(&data[cur..end]);
			cur = end;
			assert!(chunk.len() <= REPLEN);
			chunk.resize(REPLEN, 0);

			self.transport.write_chunk(chunk)?;
		}

		Ok(())
	}

	fn read(&mut self, message_type: MessageType) -> Result<Vec<u8>> {
		assert!(self.session_id != 0);

		let chunk = self.transport.read_chunk()?;
		if chunk[0] != 0x01 {
			return Err(Error::DeviceBadMagic);
		}
		if BigEndian::read_u32(&chunk[1..5]) != self.session_id {
			return Err(Error::DeviceBadSessionId);
		}
		if BigEndian::read_u32(&chunk[5..9]) != message_type as u32 {
			return Err(Error::DeviceUnexpectedMessageType);
		}

		let data_length = BigEndian::read_u32(&chunk[9..13]) as usize;
		let mut data: Vec<u8> = chunk[13..].into();

		let mut seq = 0;
		while data.len() < data_length {
			let chunk = self.transport.read_chunk()?;
			if chunk[0] != 0x02 {
				return Err(Error::DeviceBadMagic);
			}
			if BigEndian::read_u32(&chunk[1..5]) != self.session_id {
				return Err(Error::DeviceBadSessionId);
			}
			if BigEndian::read_u32(&chunk[5..9]) != seq as u32 {
				return Err(Error::DeviceUnexpectedSequenceNumber);
			}
			seq += 1;

			data.extend(&chunk[9..]);
		}

		Ok(data[0..data_length].into())
	}
}

pub struct ProtocolV1<T: Transport> {
	pub transport: T,
}

impl<T: Transport> Protocol for ProtocolV1<T> {
	fn session_begin(&mut self) -> Result<()> {
		Ok(()) // no sessions
	}

	fn session_end(&mut self) -> Result<()> {
		Ok(()) // no sessions
	}

	fn write(&mut self, message_type: MessageType, payload: Vec<u8>) -> Result<()> {
		// First generate the total payload, then write it to the transport in chunks.
		let mut data = Vec::with_capacity(8);
		data[0] = 0x23;
		data[1] = 0x23;
		BigEndian::write_u16(&mut data[2..4], message_type as u16);
		BigEndian::write_u32(&mut data[4..8], payload.len() as u32);
		data.extend(payload);

		let mut cur: usize = 0;
		while cur < data.len() {
			let mut chunk = vec![0x3f];
			let end = cmp::min(cur + (REPLEN - 1), data.len());
			chunk.extend(&data[cur..end]);
			cur = end;
			assert!(chunk.len() <= REPLEN);
			chunk.resize(REPLEN, 0);

			self.transport.write_chunk(chunk)?;
		}

		Ok(())
	}

	fn read(&mut self, message_type: MessageType) -> Result<Vec<u8>> {
		let chunk = self.transport.read_chunk()?;
		if chunk[0] != 0x3f || chunk[1] != 0x23 || chunk[2] != 0x23 {
			return Err(Error::DeviceBadMagic);
		}
		if BigEndian::read_u16(&chunk[3..5]) != message_type as u16 {
			return Err(Error::DeviceUnexpectedMessageType);
		}

		let data_length = BigEndian::read_u32(&chunk[5..9]) as usize;
		let mut data: Vec<u8> = chunk[9..].into();

		while data.len() < data_length {
			let chunk = self.transport.read_chunk()?;
			if chunk[0] != 0x3f {
				return Err(Error::DeviceBadMagic);
			}

			data.extend(&chunk[1..]);
		}

		Ok(data[0..data_length].into())
	}
}
