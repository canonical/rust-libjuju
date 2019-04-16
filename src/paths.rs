use dirs::home_dir;
use std::env;
use std::path::PathBuf;

pub fn juju_data_dir() -> PathBuf {
    env::var("JUJU_DATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            home_dir()
                .unwrap_or_else(|| PathBuf::from("/root"))
                .join(".local/share/juju")
        })
}
