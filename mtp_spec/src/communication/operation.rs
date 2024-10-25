use crate::communication::{Parameter, SessionId, TransactionId};

use alloc::vec::Vec;

use deku::DekuWrite;

mod impls;
pub use impls::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq, DekuWrite)]
pub struct Operation {
	code: u16,
	session_id: Option<SessionId>,
	transaction_id: TransactionId,
	parameters: [Option<Parameter>; 5],
}
