use mtp_spec::communication::operation::OpenSessionError;
use mtp_spec::device::Device;

pub mod error;
/// USB backend for MTP.
pub mod usb;

#[test_log::test(tokio::test)]
async fn foo() {
	for d in usb::device_list().unwrap() {
		let mut handle = d.unwrap().open().unwrap();
		let (res, mut session_id) = dbg!(handle.open_session().await.unwrap());
		if res.is_err() {
			let e = res.unwrap_err();
			match e {
				OpenSessionError::SessionAlreadyOpen(e) => {
					session_id = e.session_id;
				},
				e => panic!("{e}"),
			}
		}
		let info = dbg!(handle.get_device_info(Some(session_id)).await.unwrap()).unwrap();
		println!("{}", info.data.data.mtp_extensions);
		let storages = dbg!(handle.get_storage_ids(session_id).await.unwrap()).unwrap();
		for storage in storages.data.data {
			dbg!(handle.get_storage_info(session_id, storage).await.unwrap()).unwrap();
		}
	}
}
