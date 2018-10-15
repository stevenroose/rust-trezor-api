use hid;
use protobuf;

use error::Result;

pub trait Transport {
	fn write_chunk(&mut self, chunk: Vec<u8>) -> Result<()>;
	fn read_chunk(&mut self) -> Result<Vec<u8>>;
}

pub trait Protocol {
	fn session_begin(transport: &mut hid::Handle) -> Result<()>;
	fn session_end(transport: &mut hid::Handle) -> Result<()>;
	//fn write<M: protobuf::Message>(transport: &mut hid::Handle, message: M) -> Result<(), Error>;
	//fn read<M: protobuf::Message>(transport: &mut hid::Handle) -> Result<M, Error>;
}

pub struct ProtocolV1 {}

impl Protocol for ProtocolV1 {
	fn session_begin(transport: &mut hid::Handle) -> Result<()> {
		Ok(())
	}
	fn session_end(transport: &mut hid::Handle) -> Result<()> {
		Ok(())
	}
}

pub struct ProtocolV2 {}

impl Protocol for ProtocolV2 {
	fn session_begin(transport: &mut hid::Handle) -> Result<()> {
		Ok(())
	}
	fn session_end(transport: &mut hid::Handle) -> Result<()> {
		Ok(())
	}
}
