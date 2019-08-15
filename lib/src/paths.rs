//! Presents functions related to locating Juju directories

use std::env;
use std::path::PathBuf;

use dirs::home_dir;

/// Get a dir from an env var and subpath
fn dir_from_env(env_var: &str, suffix: PathBuf) -> PathBuf {
    env::var(env_var).map(PathBuf::from).unwrap_or_else(|_| {
        home_dir()
            .unwrap_or_else(|| PathBuf::from("/root"))
            .join(suffix)
    })
}

pub fn juju_data_dir() -> PathBuf {
    dir_from_env("JUJU_DATA", ".local/share/juju".into())
}

pub fn charm_build_dir() -> PathBuf {
    env::var("CHARM_BUILD_DIR")
        .unwrap_or_else(|_| String::from("/tmp/charm-builds"))
        .into()
}

pub fn charm_source_dir() -> PathBuf {
    dir_from_env("CHARM_SOURCE_DIR", "charms/source/".into())
}

pub fn charm_cache_dir<P: Into<PathBuf>>(charm_name: P) -> PathBuf {
    dir_from_env(
        "CHARM_CACHE_DIR",
        PathBuf::from(".cache/charm").join(charm_name.into()),
    )
}
