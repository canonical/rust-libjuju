//! Errors for juju-rs

use std::io::Error as IOError;

use failure::Fail;
use serde_yaml::Error as YamlError;

#[derive(Debug, Fail)]
pub enum JujuError {
    #[fail(display = "I/O error: {}", _0)]
    IOError(#[fail(cause)] IOError),

    #[fail(display = "YAML Error: {}", _0)]
    YamlError(YamlError),

    #[fail(display = "Failed to deserialize: {}", _0)]
    DeserError(String),

    #[fail(display = "Controller `{}` not found", _0)]
    ControllerNotFound(String),

    #[fail(display = "Model `{}` not found in controller {}", _0, _1)]
    ModelNotFound(String, String),

    #[fail(display = "Could not determine model for controller {}", _0)]
    UnknownModel(String),

    #[fail(display = "Bundle doesn't have application(s) {}", _0)]
    ApplicationNotFound(String),
}

impl From<IOError> for JujuError {
    fn from(err: IOError) -> Self {
        JujuError::IOError(err)
    }
}

impl From<YamlError> for JujuError {
    fn from(err: YamlError) -> Self {
        JujuError::YamlError(err)
    }
}
