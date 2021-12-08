//! Errors for juju-rs

use std::io::Error as IOError;

use ex::io::Error as ExIOError;
use serde_yaml::Error as YamlError;
use thiserror::Error as ThisError;
use zip::result::ZipError;

#[derive(Debug, ThisError)]
pub enum JujuError {
    #[error("I/O error: {0}")]
    IOError(#[from] IOError),

    #[error("I/O error: {0}")]
    ExIOError(#[from] ExIOError),

    #[error("YAML Error: {0}")]
    YamlError(#[from] YamlError),

    #[error("Controller `{0}` not found")]
    ControllerNotFound(String),

    #[error("No active controller found")]
    NoActiveController,

    #[error("Model `{0}` not found in controller {1}")]
    ModelNotFound(String, String),

    #[error("Could not determine model for controller {0}")]
    UnknownModel(String),

    #[error("Resource {0} not found for {1}")]
    ResourceNotFound(String, String),

    #[error("Error running subcommand `{0}`: `{1}`")]
    SubcommandError(String, String),

    #[error("Error while talking to charm store: {0}")]
    ZipError(#[from] ZipError),

    #[error("Error charm URL prefix: {0}")]
    MissingSourceError(String),
}
