use super::Folder;
use crate::error::Error;

use mtp_spec::communication::SessionId;
use mtp_spec::device::{Device, PtpIo};
use mtp_spec::error::MtpError;
use mtp_spec::object::types::ObjectFormatCode;
use mtp_spec::object::types::properties::{Name, ObjectFileName, SerializeableProperty};
use std::future::Future;

pub trait DeviceFsExt {
    fn mkdir(
        &mut self,
        session_id: SessionId,
        parent: Option<&Folder>,
        name: &str,
    ) -> impl Future<Output = Result<Folder, <Self as PtpIo>::Error>> + Send
    where
        Self: Device,
        <Self as PtpIo>::Error: From<Error>,
        <Self as PtpIo>::Error: From<MtpError>;
}

impl<D> DeviceFsExt for D
where
    D: Device,
    <D as PtpIo>::Error: From<MtpError>,
{
    async fn mkdir(
        &mut self,
        session_id: SessionId,
        parent: Option<&Folder>,
        name: &str,
    ) -> Result<Folder, <Self as PtpIo>::Error>
    where
        Self: Device,
        <Self as PtpIo>::Error: From<Error>,
        <Self as PtpIo>::Error: From<MtpError>,
    {
        // let storage = parent.map(|p| p.storage_id);
        // let parent_object = parent.map(|p| p.id);
        //
        // // TODO
        // // let prop_list = [
        // //     Box::new(ObjectFileName {}) as Box<dyn SerializeableProperty>,
        // //     Box::new(Name {}) as Box<dyn SerializeableProperty>,
        // // ];
        // self.send_object_prop_list(
        //     session_id,
        //     storage,
        //     parent_object,
        //     ObjectFormatCode::Association,
        //     0,
        //     core::iter::empty(),
        // )
        // .await?
        // .map_err(Into::<MtpError>::into)?;

        todo!()
    }
}
