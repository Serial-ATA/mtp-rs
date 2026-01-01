//! Responses to Android-specific operations
//!
//! See [`crate::communication::operation::android`]

use crate::communication::response::impls::define_response;

define_response! {
    /// Response for the [`SendPartialObject`] operation
    ///
    /// [`SendPartialObject`]: crate::communication::operation::android::SendPartialObject
    pub struct SendPartialObject[][] {
        // TODO: Ugly, should have the parameters just be bare fields so that doc comments can also be used
        parameters: (length: u32),
    }
}
