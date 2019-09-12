//! Errors for juju-rs

use std::io::Error as IOError;

use ex::io::Error as ExIOError;
use failure::Fail;
use reqwest::Error as ReqwestError;
use serde_yaml::Error as YamlError;

#[derive(Debug, Fail)]
pub enum JujuError {
    #[fail(display = "I/O error: {}", _0)]
    IOError(#[fail(cause)] IOError),

    #[fail(display = "I/O error: {}", _0)]
    ExIOError(#[fail(cause)] ExIOError),

    #[fail(display = "YAML Error: {}", _0)]
    YamlError(#[fail(cause)] YamlError),

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

    #[fail(display = "Resource {} not found for {}", _0, _1)]
    ResourceNotFound(String, String),

    #[fail(display = "Error running subcommand `{}`: `{}`", _0, _1)]
    SubcommandError(String, String),

    #[fail(
        display = "Can't promote promulgated charm name, use full charm store URL: `{}`",
        _0
    )]
    NamespaceRequired(String),

    #[fail(display = "Error while talking to network: {}", _0)]
    NetworkError(#[fail(cause)] ReqwestError),
}

impl From<IOError> for JujuError {
    fn from(err: IOError) -> Self {
        JujuError::IOError(err)
    }
}

impl From<ExIOError> for JujuError {
    fn from(err: ExIOError) -> Self {
        JujuError::ExIOError(err)
    }
}

impl From<YamlError> for JujuError {
    fn from(err: YamlError) -> Self {
        JujuError::YamlError(err)
    }
}

impl From<ReqwestError> for JujuError {
    fn from(err: ReqwestError) -> Self {
        JujuError::NetworkError(err)
    }
}
