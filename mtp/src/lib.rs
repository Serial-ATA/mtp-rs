use mtp_spec::device::Device;

pub mod error;
/// USB backend for MTP.
pub mod usb;

#[test_log::test(tokio::test)]
async fn foo() {
	for d in usb::device_list().unwrap() {
		let mut handle = d.unwrap().open().unwrap();
		dbg!(handle.get_device_info(None).await.unwrap());
	}
}
