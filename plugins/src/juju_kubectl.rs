//! Juju plugin for running `kubectl` against the current model

use clap::{clap_app, crate_authors, crate_description, crate_version};
use failure::{bail, Error};
use tempfile::NamedTempFile;

use juju::cmd::run;
use juju::local::{controller::Substrate, ControllerYaml, ModelYaml};

fn parse_model_name(model_name: &str) -> (Option<&str>, Option<&str>) {
    if model_name.is_empty() {
        return (None, None);
    }

    let split = model_name.splitn(2, ':').collect::<Vec<_>>();

    match split.len() {
        0 => (None, None),
        1 => (None, Some(split[0])),
        2 => (Some(split[0]), Some(split[1])),
        _ => unreachable!(),
    }
}

fn main() -> Result<(), Error> {
    let matches = clap_app!(("juju-kubectl") =>
        (@setting TrailingVarArg)
        (version: crate_version!())
        (author: crate_authors!())
        (about: crate_description!())
        (@arg MODEL: -m --model +takes_value "Model to operate in. Accepts [<controller name>:]<model name>")
        (@arg KUBECTL: +multiple "Arguments to pass to kubectl")
    )
        .get_matches();

    let (controller_name, model_name) = parse_model_name(matches.value_of("MODEL").unwrap_or(""));

    let controllers = ControllerYaml::load()?;
    let models = ModelYaml::load()?;

    let controller_name = controllers.validate_name(controller_name);
    let model_name = models.validate_name(&controller_name, model_name)?;

    // Get all the extra args to pass onto `kubectl`
    let mut kubectl_args = matches
        .values_of("KUBECTL")
        .map(Iterator::collect)
        .unwrap_or_else(Vec::new);

    if !kubectl_args.contains(&"-n") {
        kubectl_args.extend(&["-n", &model_name])
    }

    let substrate = controllers.substrate(&controller_name)?;

    match substrate {
        Substrate::MicroK8s => {
            run("microk8s.kubectl", &kubectl_args)?;
        }
        Substrate::CDK => {
            let kubecfg = NamedTempFile::new()?;
            let path = &kubecfg.path().as_os_str().to_string_lossy();

            let kubectl_args = vec!["--kubeconfig", path]
                .into_iter()
                .chain(kubectl_args.into_iter())
                .collect::<Vec<_>>();

            run(
                "juju",
                &[
                    "scp",
                    "-m",
                    &format!("{}:default", controller_name),
                    "kubernetes-master/0:~/config",
                    path,
                ],
            )?;

            run("kubectl", &kubectl_args)?;
        }
        Substrate::Unknown => {
            bail!("Couldn't determine cloud substrate.");
        }
    }

    Ok(())
}
