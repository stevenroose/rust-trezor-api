/// HID-related constants.
pub mod hid {
	pub const DEV_TREZOR1: (u16, u16) = (0x534C, 0x0001);
	pub const DEV_TREZOR2: (u16, u16) = (0x1209, 0x53C1);
	pub const DEV_TREZOR2_BL: (u16, u16) = (0x1209, 0x53C0);

	pub const WIRELINK_USAGE: u16 = 0xFF00;
	pub const WIRELINK_INTERFACE: isize = 0;
	pub const DEBUGLINK_USAGE: u16 = 0xFF01;
	pub const DEBUGLINK_INTERFACE: isize = 1;
}
