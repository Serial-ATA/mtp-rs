//! A FUSE filesystem for MTP-compatible devices

mod fuse;

use crate::fuse::MtpFuse;

use clap::Parser;
use fuser::{Config, MountOption, SessionACL};
use mtp::example_utils::{prompt_for_device, prompt_for_storages};
use mtp::usb::error::Error;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "mtpfs", version, about, long_about = None)]
struct Args {
    mount_point: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    let device = prompt_for_device().await?;

    let mut session = match device.open().await {
        Ok(val) => val,
        Err(e) => {
            tracing::error!("Failed to open device");
            return Err(e);
        },
    };

    let storages = prompt_for_storages(&mut session).await?;

    let mut storage_paths = Vec::with_capacity(storages.len());
    let mut sessions = Vec::new();
    for storage in storages {
        let name = storage
            .description
            .as_ref()
            .map_or_else(|| String::from("Unknown Storage"), ToString::to_string);
        let fs = MtpFuse::new(session.clone(), storage).await?;

        let target = args.mount_point.join(&name);
        if !target.exists()
            && let Err(e) = std::fs::create_dir_all(&target)
        {
            tracing::error!(
                "Failed to create mountpoint for storage `{name}` at {}: {e}",
                target.display()
            );
            return Err(Error::Io(Arc::new(e)));
        }

        storage_paths.push(target.clone());

        let mut config = Config::default();
        config.mount_options = vec![MountOption::Sync, MountOption::DefaultPermissions];
        config.acl = SessionACL::All;

        match fuser::spawn_mount2(fs, target, &config) {
            Ok(bg_session) => sessions.push(bg_session),
            Err(e) => {
                tracing::error!("Mount failed: {e}");
                return Err(Error::Io(Arc::new(e)));
            },
        }
    }

    let _ = tokio::signal::ctrl_c().await;

    tracing::info!("Shutting down");
    sessions.clear();

    for path in &storage_paths {
        if let Err(e) = std::fs::remove_dir(path) {
            tracing::error!("Failed to remove mountpoint `{}`: {e}", path.display());
        }
    }

    Ok(())
}
