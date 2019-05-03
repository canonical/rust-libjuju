//! Juju plugin for interacting with a bundle

use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;

use failure::{format_err, Error, ResultExt};
use rayon::prelude::*;
use structopt::{self, clap::AppSettings, StructOpt};
use tempfile::NamedTempFile;

use juju::parsing::bundle::{Application, Bundle};
use juju::parsing::charm::Charm;
use juju::paths;

/// CLI arguments for the `deploy` subcommand.
#[derive(StructOpt, Debug)]
struct DeployConfig {
    #[structopt(long = "recreate")]
    #[structopt(help = "Recreate the bundle by ensuring that it's removed before deploying")]
    recreate: bool,

    #[structopt(long = "build")]
    #[structopt(help = "Build the bundle before deploying it. Requires `source:` to be defined")]
    build: bool,

    #[structopt(long = "wait", default_value = "60")]
    #[structopt(help = "How long to wait in seconds for model to stabilize before deploying it")]
    wait: u32,

    #[structopt(short = "a", long = "app")]
    #[structopt(help = "Select particular apps to deploy")]
    apps: Vec<String>,

    #[structopt(short = "b", long = "bundle", default_value = "bundle.yaml")]
    #[structopt(help = "The bundle file to deploy")]
    bundle: String,

    #[structopt(name = "deploy-args")]
    #[structopt(help = "Arguments that are collected and passed on to `juju deploy`")]
    deploy_args: Vec<String>,
}

/// CLI arguments for the `remove` subcommand.
#[derive(StructOpt, Debug)]
struct RemoveConfig {
    #[structopt(short = "a", long = "app")]
    #[structopt(help = "Select particular apps to deploy")]
    apps: Vec<String>,

    #[structopt(short = "b", long = "bundle", default_value = "bundle.yaml")]
    #[structopt(help = "The bundle file to remove")]
    bundle: String,
}

/// Interact with a bundle.
#[derive(StructOpt, Debug)]
#[structopt(raw(setting = "AppSettings::TrailingVarArg"))]
#[structopt(raw(setting = "AppSettings::SubcommandRequiredElseHelp"))]
enum Config {
    /// Deploys a bundle, optionally building and/or recreating it.
    ///
    /// If a subset of apps are chosen, bundle relations are only
    /// included if both apps are selected.
    #[structopt(name = "deploy")]
    Deploy(DeployConfig),

    /// Removes a bundle from the current model.
    ///
    /// If a subset of apps are chosen, bundle relations are only
    /// included if both apps are selected.
    #[structopt(name = "remove")]
    Remove(RemoveConfig),
}

/// Run `deploy` subcommand
fn deploy(c: DeployConfig) -> Result<(), Error> {
    println!("Building and deploying bundle from {}", c.bundle);

    let mut bundle = Bundle::load(c.bundle.clone())?;

    let applications = bundle.app_subset(c.apps.clone())?;
    let build_count = applications.values().filter(|v| v.source.is_some()).count();

    println!("Found {} total applications", applications.len());
    println!("Found {} applications to build.\n", build_count);

    let temp_bundle = NamedTempFile::new()?;

    bundle.relations = bundle
        .relations
        .into_iter()
        .filter(|rels| {
            rels.iter()
                .collect::<HashSet<_>>()
                .is_subset(&applications.keys().collect())
        })
        .collect();

    // Convert a `Vec<Result<T, E>>` into `Result<Vec<T>, E>` so that any errors
    // can fail entire process, and then `Vec<T>` can be unzipped and destructured into
    // `Vec<NamedTempFile>` and `HashMap<String, Application>`.
    type AppKVPairs = (String, Application);
    let mapped: Result<Vec<(NamedTempFile, AppKVPairs)>, Error> = applications
        .par_iter()
        .map(|(name, application)| {
            let mut new_application = application.clone();
            let temp_file = NamedTempFile::new()?;

            new_application.charm = match (c.build, &application.charm, &application.source) {
                // If a charm URL was defined and either the `--build` flag wasn't passed or
                // there's no `source` property, deploy the charm URL
                (false, Some(charm), _) | (_, Some(charm), None) => Some(charm.clone()),

                // Either `charm` or `source` must be set
                (_, None, None) => {
                    return Err(format_err!(
                        "Application {} has neither `charm` nor `source` set.",
                        name
                    ));
                }

                // If the charm source was defined and either the `--build` flag was passed, or
                // if there's no `charm` property, build the charm
                (true, _, Some(source)) | (_, None, Some(source)) => {
                    println!("Building {}", name);

                    let build_dir = paths::charm_build_dir();

                    // If `source` starts with `.`, it's a relative path from the bundle we're
                    // deploying. Otherwise, look in `CHARM_SOURCE_DIR` for it.
                    let charm_path = if source.starts_with('.') {
                        PathBuf::from(&c.bundle).parent().unwrap().join(source)
                    } else {
                        paths::charm_source_dir().join(source)
                    };

                    let charm = Charm::load(&charm_path)
                        .with_context(|_| charm_path.display().to_string())?;

                    let exit_status = Command::new("charm")
                        .args(&["build", &charm_path.to_string_lossy()])
                        .args(&[
                            "--cache-dir",
                            &paths::charm_cache_dir(source).to_string_lossy(),
                        ])
                        .spawn()?
                        .wait()?;

                    if !exit_status.success() {
                        return Err(format_err!(
                            "charm build encountered an error while building {}: {}",
                            name,
                            exit_status.to_string()
                        ));
                    }

                    for (name, resource) in charm.metadata.resources {
                        if let Some(source) = resource.upstream_source {
                            new_application.resources.entry(name).or_insert(source);
                        }
                    }

                    Some(
                        build_dir
                            .join(charm.metadata.name)
                            .to_string_lossy()
                            .to_string(),
                    )
                }
            };

            Ok((temp_file, (name.clone(), new_application)))
        })
        .collect();

    let (temp_files, applications): (Vec<NamedTempFile>, _) = mapped?.into_iter().unzip();

    bundle.applications = applications;

    bundle.save(temp_bundle.path())?;

    if c.recreate {
        println!("\n\nRemoving bundle before deploy.");
        remove(RemoveConfig {
            apps: c.apps.clone(),
            bundle: c.bundle.clone(),
        })?;
    }

    if c.wait > 0 {
        println!("\n\nWaiting for stability before deploying.");

        let exit_status = Command::new("juju")
            .args(&["wait", "-wv", "-t", &c.wait.to_string()])
            .spawn()?
            .wait()?;

        if !exit_status.success() {
            return Err(format_err!(
                "Encountered an error while waiting to deploy: {}",
                exit_status.to_string()
            ));
        }
    }

    println!("\n\nDeploying bundle");

    let exit_status = Command::new("juju")
        .args(&["deploy", &temp_bundle.path().to_string_lossy()])
        .args(c.deploy_args)
        .spawn()?
        .wait()?;

    if !exit_status.success() {
        return Err(format_err!(
            "Encountered an error while deploying bundle: {}",
            exit_status.to_string()
        ));
    }

    // Force temp files to stick around for deploy command
    let _ = temp_files.len();

    Ok(())
}

/// Run `remove` subcommand
fn remove(c: RemoveConfig) -> Result<(), Error> {
    let bundle = Bundle::load(c.bundle)?;
    for name in bundle.app_subset(c.apps)?.keys() {
        Command::new("juju")
            .args(&["remove-application", name])
            .spawn()?
            .wait()?;
    }
    Ok(())
}

fn main() -> Result<(), Error> {
    match Config::from_args() {
        Config::Deploy(c) => deploy(c)?,
        Config::Remove(c) => remove(c)?,
    }

    Ok(())
}
