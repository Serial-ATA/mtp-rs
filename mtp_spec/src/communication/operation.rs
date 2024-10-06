use crate::communication::Parameter;

use alloc::vec::Vec;

use deku::DekuWrite;

mod impls;

#[derive(Copy, Clone, Debug, Eq, PartialEq, DekuWrite)]
pub struct Operation {
	code: u16,
	session_id: u32,
	transaction_id: u32,
	parameters: [Option<Parameter>; 5],
}
