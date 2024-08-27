mod impls;

pub struct Event {
	code: u16,
	session_id: u32,
	transaction_id: u32,
	parameters: [Option<u32>; 3],
}