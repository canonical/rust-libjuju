use std::io::Error as IOError;

use failure::Fail;
use serde_yaml::Error as YamlError;

#[derive(Debug, Fail)]
pub enum JujuError {
    #[fail(display = "I/O error: {}", _0)]
    IOError(IOError),

    #[fail(display = "YAML Error: {}", _0)]
    YamlError(YamlError),

    #[fail(display = "Failed to deserialize: {}", _0)]
    DeserError(String),
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

impl From<JujuError> for String {
    fn from(err: JujuError) -> Self {
        format!("{}", err)
    }
}
