use std::fs::write;
use std::process::{exit, Command};

use clap::{clap_app, crate_authors, crate_description, crate_version};
use tempfile::NamedTempFile;

use libjuju::controller::{Controllers, Substrate};

fn main() -> Result<(), String> {
    let matches = clap_app!(juju_kubectl =>
        (@setting TrailingVarArg)
        (version: crate_version!())
        (author: crate_authors!())
        (about: crate_description!())
        (@arg CONTROLLER: -c --controller +takes_value "Controller to operate in")
        (@arg KUBECTL: +multiple "Arguments to pass to kubectl")
    )
    .get_matches();

    let controllers = Controllers::load()?;
    let controller = controllers.get(matches.value_of("CONTROLLER"))?;

    // Get all the extra args to pass onto `kubectl`
    let kubectl_args = matches
        .values_of("KUBECTL")
        .map(|a| a.collect())
        .unwrap_or(vec![]);

    let kubecfg = NamedTempFile::new()
        .map_err(|err| format!("Couldn't create temporary file for kubeconfig: {}", err))?;

    let path = &kubecfg.path().as_os_str().to_string_lossy();

    match controllers.substrate(&controller.name)? {
        Substrate::MicroK8s => {
            let config = Command::new("microk8s.config")
                .output()
                .map_err(|err| format!("Got error while running kubectl: {}", err))?;

            write(kubecfg.path(), config.stdout)
                .map_err(|err| format!("Couldn't write kubeconfig: {}", err))?;
        }
        Substrate::CDK => {
            Command::new("juju")
                .args(&[
                    "scp",
                    "-m",
                    &format!("{}:default", controller.name),
                    "kubernetes-master/0:~/config",
                    path,
                ])
                .spawn()
                .map_err(|err| format!("Got error while getting kubeconfig: {}", err))?
                .wait()
                .map_err(|err| format!("Got error while getting kubeconfig: {}", err))?;
        }
        Substrate::Unknown => {
            return Err(format!(
                "Couldn't determine substrate for {}",
                controller.name
            ));
        }
    }

    let exit_status = Command::new("kubectl")
        .args(&["--kubeconfig", path])
        .args(&kubectl_args)
        .spawn()
        .map_err(|err| format!("Got error while running kubectl: {}", err))?
        .wait()
        .map_err(|err| format!("Got error while running kubectl: {}", err))?;

    exit(exit_status.code().unwrap_or(1))
}
