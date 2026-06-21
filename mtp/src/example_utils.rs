//! Utilities used in the examples

#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]

use crate::high_level::storages::{SessionStorageExt, Storage};

use dialoguer::Select;
use dialoguer::theme::ColorfulTheme;
use futures::executor::block_on_stream;
use mtp_spec::device::session::MtpSession;

/// Collect all connected MTP devices and prompt the user to select one
pub async fn prompt_for_device() -> crate::usb::error::Result<crate::usb::Device> {
    fn extract_device_name(device: &crate::usb::Device) -> String {
        match device.well_known_info() {
            Some(well_known_info) => {
                let generic_info = device.info();
                format!(
                    "   {}: {} ({:04x}:{:04x}) @ bus {}, dev {}",
                    well_known_info.vendor,
                    well_known_info.product,
                    well_known_info.vendor_id,
                    well_known_info.product_id,
                    generic_info.busnum(),
                    generic_info.device_address()
                )
            },
            None => {
                let generic_info = device.info();
                format!(
                    "   Unknown Device ({:04x}:{:04x}) @ bus {}, dev {}",
                    generic_info.vendor_id(),
                    generic_info.product_id(),
                    generic_info.busnum(),
                    generic_info.device_address()
                )
            },
        }
    }

    let mut devices = block_on_stream(crate::usb::device_list().await?)
        .filter_map(Result::ok)
        .collect::<Vec<_>>();

    if devices.is_empty() {
        tracing::error!("No devices found");
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

async fn get_storages(
    session: &mut MtpSession<crate::usb::DeviceHandle>,
    allow_multiple: bool,
) -> crate::usb::error::Result<Vec<Storage>> {
    let mut storages = session.storages().await?;

    if storages.is_empty() {
        tracing::error!(
            "No storages found. Double check that your device has allowed media access."
        );
        std::process::exit(1);
    }

    let mut storage_names = storages
        .iter()
        .map(|storage| {
            storage
                .description
                .clone()
                .unwrap_or_else(|| String::from("Unknown storage"))
        })
        .collect::<Vec<_>>();

    if allow_multiple {
        storage_names.insert(0, String::from("All"));
    }

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Which storage do you want to use?")
        .default(0)
        .items(&storage_names)
        .interact()
        .unwrap();

    if selection == 0 && allow_multiple {
        Ok(storages)
    } else {
        Ok(vec![storages.remove(selection.saturating_sub(1))])
    }
}

/// Get all storages from the device and prompt the user to select potentially many
pub async fn prompt_for_storages(
    session: &mut MtpSession<crate::usb::DeviceHandle>,
) -> crate::usb::error::Result<Vec<Storage>> {
    get_storages(session, true).await
}

/// Get all storages from the device and prompt the user to select one
pub async fn prompt_for_storage(
    session: &mut MtpSession<crate::usb::DeviceHandle>,
) -> crate::usb::error::Result<Storage> {
    Ok(get_storages(session, false)
        .await?
        .pop()
        .expect("should exist"))
}
