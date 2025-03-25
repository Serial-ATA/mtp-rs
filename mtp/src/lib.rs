use mtp_spec::device::Device;

pub mod error;
/// USB backend for MTP.
pub mod usb;

#[test_log::test(tokio::test)]
async fn foo() {
	for d in usb::device_list().unwrap() {
		let mut handle = d.unwrap().open().unwrap();
		let (res, session_id) = dbg!(handle.open_session().await.unwrap());
		res.unwrap();
		dbg!(handle.get_device_info(Some(session_id)).await.unwrap()).unwrap();
	}
}
