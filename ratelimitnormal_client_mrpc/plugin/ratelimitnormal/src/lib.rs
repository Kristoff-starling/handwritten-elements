#![feature(peer_credentials_unix_socket)]

use thiserror::Error;

pub use phoenix_common::{InitFnResult, PhoenixAddon};

pub mod config;
pub(crate) mod engine;
pub mod module;

#[derive(Error, Debug)]
pub(crate) enum DatapathError {
    #[error("Internal queue send error")]
    InternalQueueSend,
}

use phoenix_common::engine::datapath::SendError;
impl<T> From<SendError<T>> for DatapathError {
    fn from(_other: SendError<T>) -> Self {
        DatapathError::InternalQueueSend
    }
}

use crate::config::RatelimitnormalConfig;
use crate::module::RatelimitnormalAddon;

#[no_mangle]
pub fn init_addon(config_string: Option<&str>) -> InitFnResult<Box<dyn PhoenixAddon>> {
    let config = RatelimitnormalConfig::new(config_string)?;
    let addon = RatelimitnormalAddon::new(config);
    Ok(Box::new(addon))
}
