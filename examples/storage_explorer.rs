use deku::ctx::Endian;
use deku::no_std_io::Cursor;
use deku::reader::Reader;
use deku::{DekuReader, DekuWriter};
use dialoguer::Select;
use dialoguer::theme::ColorfulTheme;
use indicatif::{ProgressBar, ProgressStyle};
use mtp_spec::communication::SessionId;
use mtp_spec::communication::operation::OpenSessionError;
use mtp_spec::device::Device;
use mtp_spec::device::storage::id::StorageId;
use mtp_spec::object::types::properties::{ObjectFileName, ParentObject};
use mtp_spec::object::types::{Array, ObjectFormatCode, ObjectHandle, PtpString};

#[tokio::main]
async fn main() -> mtp::error::Result<()> {
    env_logger::init();

    let device = prompt_for_device()?;

    let mut handle = device.open()?;

    let (response, session_id) = handle.open_session().await?;
    match response {
        Ok(_) => {},
        Err(OpenSessionError::SessionAlreadyOpen(e)) => {
            log::warn!("Session {} already open", e.session_id);
        },
        Err(e) => {
            eprintln!("Failed to open session: {e}");
            std::process::exit(1)
        },
    }

    let storage = prompt_for_storage(&mut handle, session_id).await?;
    let all_objects = handle
        .get_object_handles(
            session_id,
            storage,
            Some(ObjectFormatCode::Association),
            None,
        )
        .await?;

    let objects;
    match all_objects {
        Ok(all_objects) => {
            objects = all_objects.data.data;
        },
        Err(e) => {
            eprintln!("Failed to get object handles: {e}");
            std::process::exit(1);
        },
    }

    let root_objects = collect_root_objects(&mut handle, session_id, &objects).await?;
    prompt_for_children(&mut handle, session_id, storage, root_objects).await?;

    Ok(())
}

fn walk_into_object(
    device: &mut mtp::usb::DeviceHandle,
    session_id: SessionId,
    storage_id: StorageId,
    object: mtp::object::types::ObjectHandle,
) -> impl Future<Output = mtp::error::Result<()>> {
    Box::pin(async move {
        let children = device
            .get_object_handles(session_id, storage_id, None, Some(object))
            .await
            .unwrap();
        match children {
            Ok(children) => {
                let children = children.data.data;
                let roots = collect_root_objects(device, session_id, &children)
                    .await
                    .unwrap();
                prompt_for_children(device, session_id, storage_id, roots).await
            },
            Err(e) => {
                eprintln!("Failed to get object handles: {e}");
                std::process::exit(1);
            },
        }
    })
}

fn prompt_for_children(
    device: &mut mtp::usb::DeviceHandle,
    session_id: SessionId,
    storage_id: StorageId,
    roots: Vec<(mtp::object::types::ObjectHandle, String)>,
) -> impl Future<Output = mtp::error::Result<()>> {
    Box::pin(async move {
        let root_directory_names = roots
            .iter()
            .map(|(_, name)| name.clone())
            .collect::<Vec<_>>();

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Choose a directory to step into")
            .default(0)
            .items(&root_directory_names)
            .interact()
            .unwrap();

        walk_into_object(device, session_id, storage_id, roots[selection].0).await
    })
}

async fn collect_root_objects(
    device: &mut mtp::usb::DeviceHandle,
    session_id: SessionId,
    objects: &Array<ObjectHandle>,
) -> mtp::error::Result<Vec<(ObjectHandle, String)>> {
    let bar = ProgressBar::new(objects.len() as u64)
        .with_message("Loading all directories")
        .with_style(ProgressStyle::with_template("{msg} {bar} {pos}/{len}").unwrap());

    let mut root_objects = Vec::new();
    for object in objects.iter().copied() {
        bar.inc(1);

        let parent_response = device
            .get_object_prop_value::<ParentObject>(session_id, object)
            .await?;

        let object_parent;
        match parent_response {
            Ok(parent) => {
                if parent.data.data == [0; 4] {
                    continue;
                }

                object_parent =
                    ObjectHandle::from(u32::from_be_bytes(parent.data.data.try_into().unwrap()));
            },
            Err(_) => continue,
        }

        let name_response = device
            .get_object_prop_value::<ObjectFileName>(session_id, object)
            .await?;

        match name_response {
            Ok(name) => {
                root_objects.push((
                    object,
                    PtpString::from_reader_with_ctx(
                        &mut Reader::new(Cursor::new(name.data.data)),
                        Endian::Little,
                    )
                    .unwrap()
                    .to_string(),
                ));
            },
            Err(e) => {
                eprintln!("Failed to get object name: {e}");
                std::process::exit(1);
            },
        }
    }

    Ok(root_objects)
}

fn extract_device_name(device: &mtp::usb::Device) -> String {
    device
        .info()
        .product_string()
        .map(String::from)
        .unwrap_or_else(|| String::from("Unknown device"))
}

fn prompt_for_device() -> mtp::error::Result<mtp::usb::Device> {
    let mut devices = mtp::usb::device_list()?
        .filter_map(|device| device.ok())
        .collect::<Vec<_>>();

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
