use std::fs::write;
use std::process::Command;

use clap::{clap_app, crate_authors, crate_description, crate_version};
use failure::{Error, ResultExt};
use rayon::prelude::*;
use tempfile::NamedTempFile;

use libjuju::bundle::{Application, Bundle};
use libjuju::charm::Charm;
use libjuju::paths;

fn deploy(bundle_path: &str, deploy_args: Vec<&str>) -> Result<(), Error> {
    let mut bundle = Bundle::load(bundle_path)?;

    let build_count = bundle
        .applications()
        .values()
        .filter(|v| v.source.is_some())
        .count();

    println!("Found {} total applications", bundle.applications().len());
    println!("Found {} applications to build.\n", build_count);

    let temp_bundle = NamedTempFile::new()?;

    let mapped: Result<Vec<(NamedTempFile, (String, Application))>, Error> = bundle
        .applications()
        .par_iter()
        .map(|(name, application)| {
            let mut new_application = application.clone();
            let temp_file = NamedTempFile::new()?;

            if let Some(source) = &application.source {
                println!("Building {}", name);

                let build_dir = paths::charm_build_dir();
                let source_dir = paths::charm_source_dir();
                let charm_path = source_dir.join(source);

                let charm =
                    Charm::load(&charm_path).with_context(|_| charm_path.display().to_string())?;

                Command::new("charm")
                    .args(&["build", &source_dir.join(source).to_string_lossy()])
                    .args(&[
                        "--cache-dir",
                        &paths::charm_cache_dir(source).to_string_lossy(),
                    ])
                    .spawn()?
                    .wait()?;

                new_application.charm = build_dir
                    .join(charm.metadata.name)
                    .to_string_lossy()
                    .to_string();

                for (name, resource) in charm.metadata.resources {
                    if let Some(source) = resource.upstream_source {
                        // new_application.resources.entry(name).or_insert(source);

                        // Fix for https://bugs.launchpad.net/juju/+bug/1824585
                        write(
                            temp_file.path(),
                            format!("registrypath: {}", source).as_bytes(),
                        )?;
                        new_application
                            .resources
                            .entry(name)
                            .or_insert(temp_file.path().to_string_lossy().to_string());
                    }
                }
            }
            Ok((temp_file, (name.clone(), new_application)))
        })
        .collect();

    let (temp_files, applications): (Vec<NamedTempFile>, _) = mapped?.into_iter().unzip();

    *bundle.applications_mut() = applications;

    bundle.save(temp_bundle.path())?;

    Command::new("juju")
        .args(&["deploy", &temp_bundle.path().to_string_lossy()])
        .args(deploy_args)
        .spawn()?
        .wait()?;

    // Force temp files to stick around for deploy command
    let _ = temp_files.len();

    Ok(())
}

fn remove(bundle_path: &str) -> Result<(), Error> {
    let bundle = Bundle::load(bundle_path)?;
    for name in bundle.applications().keys() {
        Command::new("juju")
            .args(&["remove-application", name])
            .spawn()?
            .wait()?;
    }
    Ok(())
}

fn main() -> Result<(), Error> {
    let matches = clap_app!(juju_buildup =>
        (@setting TrailingVarArg)
        (version: crate_version!())
        (author: crate_authors!())
        (about: crate_description!())
        (@subcommand deploy =>
            (about: "Deploys a bundle")
            (@arg BUNDLE: +takes_value +required "The bundle to build and deploy")
            (@arg recreate: --recreate "Remove bundle before deploying")
            (@arg DEPLOY: +multiple "Arguments to pass to `juju deploy`")
        )
        (@subcommand remove =>
            (about: "Removes a bundle")
            (@arg BUNDLE: +takes_value +required "The bundle to remove")
            (@arg DEPLOY: +multiple "Arguments to pass to `juju deploy`")
        )
    )
    .get_matches();

    match matches.subcommand() {
        ("deploy", Some(m)) => {
            let bundle_path = m.value_of("BUNDLE").expect("Bundle name is required.");

            if m.is_present("recreate") {
                println!("Removing bundle before deploy.");
                remove(bundle_path)?;
            }

            println!("Building and deploying bundle from {}", bundle_path);

            let deploy_args = matches
                .values_of("DEPLOY")
                .map(|a| a.collect())
                .unwrap_or(vec![]);

            deploy(bundle_path, deploy_args)
        }
        ("remove", Some(m)) => {
            let bundle_path = m.value_of("BUNDLE").expect("Bundle name is required.");

            println!("Building and deploying bundle from {}", bundle_path);

            remove(bundle_path)
        }
        _ => unreachable!(),
    }
}
