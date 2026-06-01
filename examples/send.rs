#![allow(missing_docs)]

use clap::Parser;
use mtp::example_utils::{prompt_for_device, prompt_for_storage};
use mtp::high_level::fs::FileSystem;
use mtp_spec::object::ObjectFormatCode;
use std::ffi::OsStr;
use std::path::PathBuf;
use tracing::{error, info, warn};

#[derive(clap::Parser, Debug)]
struct Args {
    /// Path to the file on the host
    src: PathBuf,
    /// Destination path of the file on the receiver
    dest: String,
}

#[tokio::main]
pub async fn main() -> mtp::usb::error::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();

    if !args.src.is_file() {
        error!("Input '{}' is not a file", args.src.display());
        std::process::exit(1);
    }

    let format = args
        .src
        .extension()
        .and_then(OsStr::to_str)
        .map(ObjectFormatCode::from_extension)
        .unwrap_or(ObjectFormatCode::Undefined);
    if format == ObjectFormatCode::Undefined {
        warn!(
            "Unable to determine the file type by the path, the device may reject unknown file \
             types"
        );
    }

    let file_contents;
    match std::fs::read(&args.src) {
        Ok(c) => file_contents = c,
        Err(e) => {
            error!("Failed to read '{}': {e}", args.src.display());
            std::process::exit(1);
        },
    }

    let device;
    match prompt_for_device().await {
        Ok(d) => device = d,
        Err(e) => {
            error!("Failed to select device: {e}");
            return Err(e);
        },
    }

    let mut session = match device.open().await {
        Ok(val) => val,
        Err(e) => {
            error!("Failed to open device");
            return Err(e);
        },
    };

    let storage = prompt_for_storage(&mut session).await?;
    info!("Loading filesystem...");
    let (fs, _fs_events) = FileSystem::load(session, storage.id).await?;

    let file;
    match fs.create(args.dest, format, file_contents).await {
        Ok(f) => file = f,
        Err(e) => {
            error!("Failed to create object on device: {e}");
            std::process::exit(1);
        },
    }

    info!(
        "File successfully created on device (handle: {:#X})",
        Into::<u32>::into(file.id())
    );
    Ok(())
}
