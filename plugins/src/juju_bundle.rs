//! Juju plugin for interacting with a bundle

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Command;

use ex::fs;
use failure::{format_err, Error, ResultExt};
use petgraph::{
    dot::{Config as GraphConfig, Dot},
    Graph,
};
use rayon::prelude::*;
use structopt::{self, clap::AppSettings, StructOpt};
use tempfile::{NamedTempFile, TempDir};

use juju::bundle::{Application, Bundle};
use juju::channel::Channel;
use juju::charm_source::CharmSource;
use juju::cmd::run;
use juju::paths;

/// CLI arguments for the `deploy` subcommand.
#[derive(StructOpt, Debug)]
struct DeployConfig {
    #[structopt(long = "recreate")]
    #[structopt(help = "Recreate the bundle by ensuring that it's removed before deploying")]
    recreate: bool,

    #[structopt(long = "upgrade-charms")]
    #[structopt(help = "Runs upgrade-charm on each individual charm instead of redeploying")]
    upgrade_charms: bool,

    #[structopt(long = "build")]
    #[structopt(help = "Build the bundle before deploying it. Requires `source:` to be defined")]
    build: bool,

    #[structopt(long = "wait", default_value = "60")]
    #[structopt(help = "How long to wait in seconds for model to stabilize before deploying it")]
    wait: u32,

    #[structopt(short = "a", long = "app")]
    #[structopt(help = "Select particular apps to deploy")]
    apps: Vec<String>,

    #[structopt(short = "e", long = "except")]
    #[structopt(help = "Select particular apps to skip deploying")]
    exceptions: Vec<String>,

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
    #[structopt(help = "Select particular apps to remove")]
    apps: Vec<String>,

    #[structopt(short = "b", long = "bundle", default_value = "bundle.yaml")]
    #[structopt(help = "The bundle file to remove")]
    bundle: String,
}

/// CLI arguments for the `publish` subcommand.
#[derive(StructOpt, Debug)]
struct PublishConfig {
    #[structopt(short = "b", long = "bundle", default_value = "bundle.yaml")]
    #[structopt(help = "The bundle file to publish")]
    bundle: String,

    #[structopt(long = "url")]
    #[structopt(help = "The charm store URL for the bundle")]
    cs_url: String,

    #[structopt(long = "publish-charms")]
    #[structopt(help = "If set, charms will be built and published")]
    publish_charms: bool,

    #[structopt(long = "publish-namespace")]
    #[structopt(help = "If set, the namespace to publish charms under")]
    publish_namespace: Option<String>,

    #[structopt(long = "serial")]
    #[structopt(help = "If set, only one charm will be built and published at a time")]
    serial: bool,

    #[structopt(long = "prune")]
    #[structopt(
        help = "If set, docker will be pruned between each charm. Enforces --serial also set."
    )]
    prune: bool,
}

/// CLI arguments for the `publish` subcommand.
#[derive(StructOpt, Debug)]
struct PromoteConfig {
    #[structopt(short = "b", long = "bundle")]
    #[structopt(help = "The bundle file to promote")]
    bundle: String,

    #[structopt(long = "from")]
    #[structopt(help = "The bundle channel to promote from")]
    from: Channel,

    #[structopt(long = "to")]
    #[structopt(help = "The bundle channel to promote to")]
    to: Channel,

    #[structopt(short = "a", long = "application")]
    #[structopt(help = "Select apps to promote with the bundle")]
    apps: Vec<String>,
}

/// CLI arguments for the `export` subcommand.
#[derive(StructOpt, Debug)]
struct ExportConfig {
    #[structopt(short = "b", long = "bundle", default_value = "bundle.yaml")]
    #[structopt(help = "The bundle file to export")]
    bundle: String,

    #[structopt(short = "o", long = "out")]
    #[structopt(help = "Where to write the exported bundle")]
    out: Option<String>,
}

/// Interact with a bundle and the charms contained therein.
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

    /// Publishes a bundle and its charms to the charm store
    ///
    /// Publishes them to the edge channel. To migrate the bundle
    /// and its charms to other channels, use `juju bundle promote`.
    #[structopt(name = "publish")]
    Publish(PublishConfig),

    /// Promotes a bundle and its charms from one channel to another
    #[structopt(name = "promote")]
    Promote(PromoteConfig),

    /// Exports the bundle to different formats, e.g. graphviz
    #[structopt(name = "export")]
    Export(ExportConfig),
}

/// Run `deploy` subcommand
fn deploy(c: DeployConfig) -> Result<(), Error> {
    println!("Building and deploying bundle from {}", c.bundle);

    let mut bundle = Bundle::load(c.bundle.clone())?;

    let applications = bundle.app_subset(c.apps.clone(), c.exceptions.clone())?;
    let build_count = applications
        .iter()
        .filter(|(name, app)| app.source(name, &c.bundle).is_some())
        .count();

    println!("Found {} total applications", applications.len());
    println!("Found {} applications to build.\n", build_count);

    let temp_bundle = NamedTempFile::new()?;

    // Filter out relations that point to an application that was filtered out
    bundle.relations = bundle
        .relations
        .into_iter()
        .filter(|rels| {
            // Strip out interface name-style syntax before filtering,
            // e.g. `foo:bar` => `foo`.
            rels.iter()
                .map(|r| r.split(':').next().unwrap())
                .collect::<HashSet<_>>()
                .is_subset(&applications.keys().map(String::as_ref).collect())
        })
        .collect();

    let mapped: Result<HashMap<String, Application>, Error> = applications
        .par_iter()
        .map(|(name, application)| {
            let mut new_application = application.clone();

            let source = application.source(name, &c.bundle);
            new_application.charm = match (c.build, &application.charm, source) {
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

                    // If `source` starts with `.`, it's a relative path from the bundle we're
                    // deploying. Otherwise, look in `CHARM_SOURCE_DIR` for it.
                    let charm_path = if source.starts_with('.') {
                        PathBuf::from(&c.bundle).parent().unwrap().join(source)
                    } else {
                        paths::charm_source_dir().join(source)
                    };

                    let charm = CharmSource::load(&charm_path)?;

                    charm.build(name)?;

                    for (name, resource) in &charm.metadata.resources {
                        if let Some(source) = &resource.upstream_source {
                            new_application
                                .resources
                                .entry(name.into())
                                .or_insert_with(|| source.into());
                        }
                    }

                    Some(charm.artifact_path())
                }
            };

            Ok((name.clone(), new_application))
        })
        .collect();

    bundle.applications = mapped?;

    // If we're only upgrading charms, we can skip the rest of the logic
    // that is concerned with tearing down and/or deploying the charms.
    if c.upgrade_charms {
        return Ok(bundle.upgrade_charms()?);
    }

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

    Ok(())
}

/// Run `remove` subcommand
fn remove(c: RemoveConfig) -> Result<(), Error> {
    let bundle = Bundle::load(c.bundle)?;
    for name in bundle.app_subset(c.apps, vec![])?.keys() {
        Command::new("juju")
            .args(&["remove-application", name])
            .spawn()?
            .wait()?;
    }
    Ok(())
}

/// Run `publish` subcommand
fn publish(c: PublishConfig) -> Result<(), Error> {
    if c.prune && !c.serial {
        return Err(format_err!(
            "To use --prune, you must set the --serial flag as well."
        ));
    }
    let path = c.bundle.as_str();
    let bundle = Bundle::load(path)?;

    // Make sure we're logged in first, so that we don't get a bunch of
    // login pages spawn with `charm push`.
    println!("Logging in to charm store, this may open up a browser window.");
    run("charm", &["login"])?;

    let revisions: Result<Vec<(String, String)>, Error> = if c.publish_charms {
        let publish_handler = |(name, app): (&String, &Application)| {
            match (&app.charm, &app.source(name, &c.bundle)) {
                (Some(cs_url), Some(source)) => {
                    // If `source` starts with `.`, it's a relative path from the bundle we're
                    // deploying. Otherwise, look in `CHARM_SOURCE_DIR` for it.
                    let charm_path = if source.starts_with('.') {
                        PathBuf::from(path).parent().unwrap().join(source)
                    } else {
                        paths::charm_source_dir().join(source)
                    };

                    let charm = CharmSource::load(&charm_path)
                        .with_context(|_| charm_path.display().to_string())?;

                    charm.build(name)?;
                    let rev_url = charm.push(
                        &cs_url
                            .with_namespace(c.publish_namespace.clone())
                            .to_string(),
                        &app.resources,
                    )?;

                    charm.promote(&rev_url, Channel::Edge)?;

                    if c.prune {
                        run("docker", &["system", "prune", "-af"])?;
                    }

                    Ok((name.clone(), rev_url))
                }
                (Some(charm), None) => {
                    let revision = charm.show(Channel::Stable)?.id_revision.revision;
                    Ok((
                        name.clone(),
                        charm.with_revision(Some(revision)).to_string(),
                    ))
                }
                (None, _) => Err(format_err!("Charm URL required: {}", name)),
            }
        };

        // Build each charm, upload it to the store, then promote that
        // revision to edge. Return a list of the revision URLs, so that
        // we can generate a bundle with those exact revisions to upload.
        if c.serial {
            bundle.applications.iter().map(publish_handler).collect()
        } else {
            bundle
                .applications
                .par_iter()
                .map(publish_handler)
                .collect()
        }
    } else {
        bundle
            .applications
            .par_iter()
            .map(|(name, app)| match &app.charm {
                Some(charm) => {
                    let channel = if app.source(name, &c.bundle).is_some() {
                        Channel::Edge
                    } else {
                        Channel::Stable
                    };
                    let revision = charm.show(channel)?.id_revision.revision;
                    Ok((
                        name.clone(),
                        charm.with_revision(Some(revision)).to_string(),
                    ))
                }
                None => Err(format_err!("Charm URL required: {}", name)),
            })
            .collect()
    };

    // Make a copy of the bundle with exact revisions of each charm
    let mut new_bundle = bundle.clone();

    for (name, revision) in revisions? {
        new_bundle
            .applications
            .get_mut(&name)
            .expect("App must exist!")
            .charm = Some(revision.parse().unwrap());
    }

    // Create a temp dir for the bundle to point `charm` at,
    // since we don't want to modify the existing bundle.yaml file.
    let dir = TempDir::new()?;
    new_bundle.save(dir.path().join("bundle.yaml"))?;

    // `charm push` expects this file to exist
    fs::copy(
        PathBuf::from(c.bundle).with_file_name("README.md"),
        dir.path().join("README.md"),
    )?;

    let bundle_url = bundle.push(dir.path().to_string_lossy().as_ref(), &c.cs_url)?;

    bundle.release(&bundle_url, Channel::Edge)?;

    Ok(())
}

/// Run `promote` subcommand
fn promote(c: PromoteConfig) -> Result<(), Error> {
    let (revision, bundle) = Bundle::load_from_store(&c.bundle, c.from)?;

    println!("Found bundle revision {}", revision);

    for (name, app) in &bundle.applications {
        if !c.apps.contains(name) {
            continue;
        }
        println!("Promoting {} to {:?}.", name, c.to);
        app.release(c.to)?;
    }

    println!("Bundle charms successfully promoted, promoting bundle.");

    bundle.release(&format!("{}-{}", c.bundle, revision), c.to)?;

    Ok(())
}

/// Run `export` subcommand
fn export(c: ExportConfig) -> Result<(), Error> {
    let bundle = Bundle::load(&c.bundle)?;

    let mut graph = Graph::<_, String>::new();

    for app in bundle.applications.keys() {
        graph.add_node(app);
    }
    for rel in bundle.relations {
        let app_a = rel[0].split(":").next().unwrap_or(&rel[0]);
        let app_b = rel[1].split(":").next().unwrap_or(&rel[1]);
        let rel_name = rel[0].split(":").last().unwrap_or("");
        let index_a = graph.node_indices().find(|i| graph[*i] == app_a).unwrap();
        let index_b = graph.node_indices().find(|i| graph[*i] == app_b).unwrap();
        graph.add_edge(index_a, index_b, rel_name.to_string());
    }
    let output = Dot::with_config(&graph, &[GraphConfig::EdgeNoLabel]);

    match c.out {
        Some(out) => fs::write(out, format!("{}", output))?,
        None => println!("{}", output),
    }

    Ok(())
}

fn main() -> Result<(), Error> {
    match Config::from_args() {
        Config::Deploy(c) => deploy(c),
        Config::Remove(c) => remove(c),
        Config::Publish(c) => publish(c),
        Config::Promote(c) => promote(c),
        Config::Export(c) => export(c),
    }
}
