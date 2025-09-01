#![allow(missing_docs)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use std::ffi::{c_int, CStr};
use std::mem::offset_of;
use std::slice;

use deku::DekuContainerRead;
use mtp::communication::operation::Operation;
use mtp::usb::{ContainerType, UsbContainer};

fn usb_container_type_c(ty: ContainerType) -> &'static CStr {
	match ty {
		ContainerType::Undefined => c"Undefined",
		ContainerType::Command => c"Command",
		ContainerType::Data => c"Data",
		ContainerType::Response => c"Response",
		ContainerType::Event => c"Event",
	}
}

fn opcode_name_c(op: Operation) -> &'static CStr {
	match op {
		Operation::GetDeviceInfo => c"GetDeviceInfo",
		Operation::OpenSession => c"OpenSession",
		Operation::CloseSession => c"CloseSession",
		Operation::GetStorageIDs => c"GetStorageIDs",
		Operation::GetStorageInfo => c"GetStorageInfo",
		Operation::GetNumObjects => c"GetNumObjects",
		Operation::GetObjectHandles => c"GetObjectHandles",
		Operation::GetObjectInfo => c"GetObjectInfo",
		Operation::GetObject => c"GetObject",
		Operation::GetThumb => c"GetThumb",
		Operation::DeleteObject => c"DeleteObject",
		Operation::SendObjectInfo => c"SendObjectInfo",
		Operation::SendObject => c"SendObject",
		Operation::InitiateCapture => c"InitiateCapture",
		Operation::FormatStore => c"FormatStore",
		Operation::ResetDevice => c"ResetDevice",
		Operation::SelfTest => c"SelfTest",
		Operation::SetObjectProtection => c"SetObjectProtection",
		Operation::PowerDown => c"PowerDown",
		Operation::GetDevicePropDesc => c"GetDevicePropDesc",
		Operation::GetDevicePropValue => c"GetDevicePropValue",
		Operation::SetDevicePropValue => c"SetDevicePropValue",
		Operation::ResetDevicePropValue => c"ResetDevicePropValue",
		Operation::TerminateOpenCapture => c"TerminateOpenCapture",
		Operation::MoveObject => c"MoveObject",
		Operation::CopyObject => c"CopyObject",
		Operation::GetPartialObject => c"GetPartialObject",
		Operation::InitiateOpenCapture => c"InitiateOpenCapture",
		Operation::GetObjectPropsSupported => c"GetObjectPropsSupported",
		Operation::GetObjectPropDesc => c"GetObjectPropDesc",
		Operation::GetObjectPropValue => c"GetObjectPropValue",
		Operation::SetObjectPropValue => c"SetObjectPropValue",
		Operation::GetObjectReferences => c"GetObjectReferences",
		Operation::SetObjectReferences => c"SetObjectReferences",
		Operation::Skip => c"Skip",
		Operation::GetObjectPropList => c"GetObjectPropList",
		Operation::SetObjectPropList => c"SetObjectPropList",
		Operation::GetInterdependentPropDesc => c"GetInterdependentPropDesc",
		Operation::SendObjectPropList => c"SendObjectPropList",
		Operation::VendorSpecific(_) => c"Vendor Specific",
	}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mtp_parse_usb(packet: *const u8, len: u32, parsed: *mut parsed_container) -> bool {
    let bytes = unsafe { slice::from_raw_parts(packet, len as usize) };
	let Ok((_, container)) = UsbContainer::from_bytes((bytes, 0)) else {
		return false;
	};

	let Ok(opcode) = Operation::try_from(container.code) else {
		return false;
	};

	unsafe {
		let UsbContainer {
			length,
			type_,
			code: _,
			transaction_id,
			payload: _
		} = container;

		{
			(*parsed).container_len = length;
			(*parsed).container_len_start = offset_of!(UsbContainer, length) as c_int;
			(*parsed).container_len_len = size_of_val(&length) as c_int;
		}

		{
			let val = usb_container_type_c(type_);
			(*parsed).container_type = val.as_ptr();
			(*parsed).container_type_start = offset_of!(UsbContainer, type_) as c_int;
			(*parsed).container_type_type_len = size_of_val(&type_) as c_int;
			(*parsed).container_type_str_len = val.to_bytes().len() as c_int;
		}

		{
			let val = opcode_name_c(opcode);
			(*parsed).opcode = val.as_ptr();
			(*parsed).opcode_start = offset_of!(UsbContainer, code) as c_int;
			(*parsed).opcode_type_len = size_of::<u16>() as c_int;
			(*parsed).opcode_str_len = val.to_bytes().len() as c_int;
		}

		{
			(*parsed).transaction_id = transaction_id.value();
			(*parsed).transaction_id_start = offset_of!(UsbContainer, transaction_id) as c_int;
			(*parsed).transaction_id_len = size_of_val(&transaction_id) as c_int;
		}
	}

	true
}
