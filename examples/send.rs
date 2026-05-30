use clap::Parser;
use dialoguer::Select;
use dialoguer::theme::ColorfulTheme;
use futures::executor::block_on_stream;
use mtp::example_utils::{prompt_for_device, prompt_for_storage};
use mtp::high_level::fs::FileSystem;
use mtp_spec::device::Device;
use mtp_spec::object::types::ObjectFormatCode;
use std::ffi::OsStr;
use std::path::PathBuf;

#[derive(clap::Parser, Debug)]
struct Args {
    /// Path to the file on the host
    src: PathBuf,
    /// Destination path of the file on the receiver
    dest: String,
}

#[tokio::main]
pub async fn main() -> mtp::usb::error::Result<()> {
    let args = Args::parse();

    if !args.src.is_file() {
        eprintln!("[ERROR] Input '{}' is not a file", args.src.display());
        std::process::exit(1);
    }

    let format = args
        .src
        .extension()
        .and_then(OsStr::to_str)
        .and_then(ObjectFormatCode::from_extension)
        .unwrap_or(ObjectFormatCode::Undefined);
    if format == ObjectFormatCode::Undefined {
        eprintln!(
            "[WARN] Unable to determine the file type by the path, the device may reject unknown \
             file types"
        );
    }

    let file_contents;
    match std::fs::read(&args.src) {
        Ok(c) => file_contents = c,
        Err(e) => {
            eprintln!("[ERROR] Failed to read '{}': {e}", args.src.display());
            std::process::exit(1);
        },
    }

    let device;
    match prompt_for_device().await {
        Ok(d) => device = d,
        Err(e) => {
            eprintln!("[ERROR] Failed to select device: {e}");
            return Err(e);
        },
    }

    let mut session = match device.open().await {
        Ok(val) => val,
        Err(e) => {
            eprintln!("[ERROR] Failed to open device");
            return Err(e);
        },
    };

    let storage = prompt_for_storage(&mut session).await?;
    let mut fs = FileSystem::load(&mut session, storage.id).await?;

    let file;
    match fs
        .create(&mut session, args.dest, format, file_contents)
        .await
    {
        Ok(f) => file = f,
        Err(e) => {
            eprintln!("[ERROR] Failed to create object on device: {e}");
            std::process::exit(1);
        },
    }

    eprintln!(
        "[INFO] File successfully created on device (handle: {})",
        Into::<u32>::into(file.id)
    );
    Ok(())
}
