use std::ffi::OsStr;
use std::process::Command;

use crate::error::JujuError;

pub fn run<S: AsRef<OsStr>>(cmd: &str, args: &[S]) -> Result<(), JujuError> {
    let status = Command::new(cmd)
        .args(args)
        .env("CHARMCRAFT_DEVELOPER", "y")
        .spawn()
        .map_err(|err| JujuError::SubcommandError(cmd.to_string(), err.to_string()))?
        .wait()
        .map_err(|err| JujuError::SubcommandError(cmd.to_string(), err.to_string()))?;

    if status.success() {
        Ok(())
    } else {
        Err(JujuError::SubcommandError(
            format!(
                "`{} {}`",
                cmd,
                args.iter()
                    .map(|a| a.as_ref().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            status.to_string(),
        ))
    }
}

pub fn get_output<S: AsRef<OsStr>>(cmd: &str, args: &[S]) -> Result<Vec<u8>, JujuError> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|err| JujuError::SubcommandError(cmd.to_string(), err.to_string()))?;

    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(JujuError::SubcommandError(
            format!(
                "`{} {}`",
                cmd,
                args.iter()
                    .map(|a| a.as_ref().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }
}

pub fn get_stderr<S: AsRef<OsStr>>(cmd: &str, args: &[S]) -> Result<Vec<u8>, JujuError> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|err| JujuError::SubcommandError(cmd.to_string(), err.to_string()))?;

    if output.status.success() {
        Ok(output.stderr)
    } else {
        Err(JujuError::SubcommandError(
            format!(
                "`{} {}`",
                cmd,
                args.iter()
                    .map(|a| a.as_ref().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }
}
