use dialoguer::Select;
use dialoguer::theme::ColorfulTheme;
use futures::executor::block_on_stream;
use mtp::communication::SessionId;
use mtp::high_level::storages::{DeviceStorageExt, Storage};

pub async fn prompt_for_device() -> mtp::error::Result<mtp::usb::Device> {
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

    let mut devices = block_on_stream(mtp::usb::device_list().await?)
        .filter_map(Result::ok)
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

pub async fn prompt_for_storages(
    device: &mut mtp::usb::DeviceHandle,
    session_id: SessionId,
) -> mtp::error::Result<Vec<Storage>> {
    let mut storages = match device.storages(session_id).await {
        Ok(storages) => storages,
        Err(e) => {
            eprintln!("Failed to get storage list: {e}");
            std::process::exit(1);
        },
    };

    if storages.is_empty() {
        log::error!("No storages found. Double check that your device has allowed media access.");
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

    storage_names.insert(0, String::from("All"));

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Which storage do you want to use?")
        .default(0)
        .items(&storage_names)
        .interact()
        .unwrap();

    if selection == 0 {
        Ok(storages)
    } else {
        Ok(vec![storages.remove(selection - 1)])
    }
}
