mod error;
pub mod usb;

#[test]
fn foo() {
	for d in usb::device_list().unwrap() {
		println!("{:#?}", d);
		d.unwrap().open().unwrap()
	}
}
