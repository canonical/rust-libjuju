use crate::error::JujuError;
use std::process::Command;

pub fn run(cmd: &str, args: &[&str]) -> Result<(), JujuError> {
    let status = Command::new(cmd).args(args).spawn()?.wait()?;

    if status.success() {
        Ok(())
    } else {
        Err(JujuError::SubcommandError(
            format!("`{} {}`", cmd, args.join(" ")),
            status.to_string(),
        ))
    }
}

pub fn get_output(cmd: &str, args: &[&str]) -> Result<Vec<u8>, JujuError> {
    let output = Command::new(cmd).args(args).output()?;

    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(JujuError::SubcommandError(
            format!("`{} {}`", cmd, args.join(" ")),
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }
}
