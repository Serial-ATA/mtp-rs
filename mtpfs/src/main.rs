mod fuse;

use crate::fuse::MtpFuse;
use dialoguer::Select;
use dialoguer::theme::ColorfulTheme;
use mtp::communication::SessionId;
use mtp::device::Device;
use mtp::device::storage::id::StorageId;
use mtp::error::Error;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::init();

    let device = prompt_for_device()?;

    let (mut handle, session_id) = match device.open().await {
        Ok(val) => val,
        Err(e) => {
            log::error!("Failed to open device");
            return Err(e);
        },
    };

    let storage = prompt_for_storage(&mut handle, session_id).await?;

    let storage_info;
    match handle.get_storage_info(session_id, storage).await? {
        Ok(info) => {
            storage_info = info.data.data;
        },
        Err(e) => {
            log::error!("Failed to get storage info: {e}");
            return Err(Error::Generic(e.into()));
        },
    }

    let fs = MtpFuse::new(handle, storage_info);

    let mp = Path::new("/home/alex/mount_phone");
    fuser::mount2(fs, mp, &[])?;

    Ok(())
}

fn prompt_for_device() -> mtp::error::Result<mtp::usb::Device> {
    fn extract_device_name(device: &mtp::usb::Device) -> String {
        match device.well_known_info() {
            Some(well_known_info) => {
                let generic_info = device.info();
                format!(
                    "   {}: {} ({:04x}:{:04x}) @ bus {}, dev {}",
                    well_known_info.vendor,
                    well_known_info.product,
                    well_known_info.vendor_id,
                    well_known_info.product_id,
                    generic_info.bus_number(),
                    generic_info.device_address()
                )
            },
            None => {
                let generic_info = device.info();
                format!(
                    "   Unknown Device ({:04x}:{:04x}) @ bus {}, dev {}",
                    generic_info.vendor_id(),
                    generic_info.product_id(),
                    generic_info.bus_number(),
                    generic_info.device_address()
                )
            },
        }
    }

    let mut devices = mtp::usb::device_list()?
        .filter_map(|device| device.ok())
        .collect::<Vec<_>>();

    if devices.is_empty() {
        log::error!("No devices found");
        std::process::exit(1);
    }

    let device_names = devices.iter().map(extract_device_name).collect::<Vec<_>>();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Which device do you want to use?")
        .default(0)
        .items(&device_names)
        .interact()
        .unwrap();

    Ok(devices.remove(selection))
}

async fn prompt_for_storage(
    device: &mut mtp::usb::DeviceHandle,
    session_id: SessionId,
) -> mtp::error::Result<StorageId> {
    let response = device.get_storage_ids(session_id).await?;

    let storage_ids;
    match response {
        Ok(storages_list) => {
            storage_ids = storages_list.data.data;
        },
        Err(e) => {
            eprintln!("Failed to get storage list: {e}");
            std::process::exit(1);
        },
    }

    if storage_ids.is_empty() {
        log::error!("No storages found. Double check that your device has allowed media access.");
        std::process::exit(1);
    }

    let mut storages = Vec::with_capacity(storage_ids.len());
    for storage_id in storage_ids.iter().copied() {
        let response = device.get_storage_info(session_id, storage_id).await?;
        match response {
            Ok(storage_info) => {
                storages.push(storage_info.data.data);
            },
            Err(e) => {
                eprintln!("Failed to get storage info: {e}");
                std::process::exit(1);
            },
        }
    }

    let storage_names = storages
        .iter()
        .map(|storage| {
            storage
                .storage_description
                .as_ref()
                .map(|storage| storage.to_string())
                .unwrap_or_else(|| String::from("Unknown storage"))
        })
        .collect::<Vec<_>>();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Which storage do you want to use?")
        .default(0)
        .items(&storage_names)
        .interact()
        .unwrap();

    let storage = storage_ids[selection];

    Ok(storage)
}
