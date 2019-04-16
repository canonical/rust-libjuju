use std::fs::write;
use std::process::Command;

use clap::{clap_app, crate_authors, crate_description, crate_version};
use tempfile::{NamedTempFile, TempDir};

use libjuju::bundle::Bundle;
use libjuju::charm::Charm;

fn main() -> Result<(), String> {
    let matches = clap_app!(juju_buildup =>
        (@setting TrailingVarArg)
        (version: crate_version!())
        (author: crate_authors!())
        (about: crate_description!())
        (@arg BUNDLE: +takes_value +required "The bundle to build and deploy")
        (@arg DEPLOY: +multiple "Arguments to pass to `juju deploy`")
    )
    .get_matches();

    let bundle_path = matches
        .value_of("BUNDLE")
        .expect("Bundle name is required.");

    let mut bundle = Bundle::load(bundle_path)?;

    let build_count = bundle
        .applications()
        .values()
        .filter(|v| v.build.is_some())
        .count();

    println!("Building and deploying bundle from {}", bundle_path);
    println!("Found {} total applications", bundle.applications().len());
    println!("Found {} applications to build.\n", build_count);

    let temp_bundle =
        NamedTempFile::new().map_err(|err| format!("Couldn't create temporary file: {}", err))?;

    // Store temp dirs here so that they get destroyed at the end of main's scope
    let mut temp_dirs = Vec::new();
    let mut temp_image_names = Vec::new();

    for (name, application) in bundle.applications_mut() {
        if let Some(b) = &application.build {
            println!("Building {}", name);

            let charm = Charm::load(b)?;

            temp_dirs
                .push(TempDir::new().map_err(|err| format!("Couldn't create temp dir: {}", err))?);
            let dir = temp_dirs.last().expect("last temp_dir can't not exist.");

            Command::new("charm")
                .args(&["build", "--build-dir", &dir.path().to_string_lossy(), b])
                .spawn()
                .map_err(|err| format!("Couldn't build charm: {}", err))?
                .wait()
                .map_err(|err| format!("Couldn't build charm: {}", err))?;

            application.charm = dir
                .path()
                .join(charm.metadata.name)
                .to_string_lossy()
                .to_string();

            for (name, resource) in charm.metadata.resources {
                if let Some(source) = resource.upstream_source {
                    // application.resources.entry(name).or_insert(source);

                    // Fix for https://bugs.launchpad.net/juju/+bug/1824585
                    temp_image_names.push(NamedTempFile::new().unwrap());
                    let temp_image_file = temp_image_names
                        .last()
                        .expect("last temp image name can't not exist.");
                    write(
                        temp_image_file.path(),
                        format!("registrypath: {}", source).as_bytes(),
                    )
                    .unwrap();
                    application
                        .resources
                        .entry(name)
                        .or_insert(temp_image_file.path().to_string_lossy().to_string());
                }
            }

            println!();
        }
    }

    bundle.save(temp_bundle.path())?;

    let deploy_args = matches
        .values_of("DEPLOY")
        .map(|a| a.collect())
        .unwrap_or(vec![]);

    Command::new("juju")
        .args(&["deploy", &temp_bundle.path().to_string_lossy()])
        .args(deploy_args)
        .spawn()
        .map_err(|err| format!("Couldn't deploy bundle: {}", err))?
        .wait()
        .map_err(|err| format!("Couldn't deploy bundle: {}", err))?;

    Ok(())
}
