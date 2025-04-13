mod fuse;
mod prompts;

use crate::fuse::MtpFuse;
use fuser::MountOption;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use mtp::error::Error;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::init();

    let mount_point = Path::new("/home/alex/mount");

    let device = prompts::prompt_for_device()?;

    let (mut handle, session_id) = match device.open().await {
        Ok(val) => val,
        Err(e) => {
            log::error!("Failed to open device");
            return Err(e);
        },
    };

    let storages = prompts::prompt_for_storages(&mut handle, session_id).await?;

    let device = Arc::new(Mutex::new(handle));

    let mut storage_paths = Vec::with_capacity(storages.len());
    let mut sessions = FuturesUnordered::new();
    for storage in storages {
        let name = storage
            .description
            .as_ref()
            .map_or_else(|| String::from("Unknown Storage"), ToString::to_string);
        let fs = MtpFuse::new(device.clone(), session_id, storage);

        let target = mount_point.join(&name);
        if !target.exists() {
            if let Err(e) = std::fs::create_dir_all(&target) {
                log::error!(
                    "Failed to create mountpoint for storage `{name}` at {}: {e}",
                    target.display()
                );
                return Err(Error::Io(e));
            }
        }

        storage_paths.push(target.clone());

        sessions.push(tokio::task::spawn(async move {
            if let Err(e) = fuser::mount2(
                fs,
                target,
                &[
                    MountOption::AutoUnmount,
                    MountOption::AllowOther,
                    MountOption::Sync,
                ],
            ) {
                log::error!("Mount failed: {e}");
            }
        }));
    }

    loop {
        tokio::select! {
            _ = sessions.next() => {
                sessions.clear();

                for path in &storage_paths {
                    if let Err(e) = std::fs::remove_dir_all(path) {
                        log::error!("Failed to remove mountpoint `{}`: {e}", path.display());
                    }
                }

                std::process::exit(1);
            },
            _ = tokio::signal::ctrl_c() => {
                log::info!("Shutting down");
                sessions.clear();

                for path in &storage_paths {
                    if let Err(e) = std::fs::remove_dir_all(path) {
                        log::error!("Failed to remove mountpoint `{}`: {e}", path.display());
                    }
                }

                break;
            }
        }
    }

    Ok(())
}
