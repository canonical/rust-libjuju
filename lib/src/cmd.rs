use crate::error::JujuError;
use std::ffi::OsStr;
use std::process::Command;

pub fn run<S: AsRef<OsStr>>(cmd: &str, args: &[S]) -> Result<(), JujuError> {
    let status = Command::new(cmd).args(args).spawn()?.wait()?;

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
    let output = Command::new(cmd).args(args).output()?;

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
