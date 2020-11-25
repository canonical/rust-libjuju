//! Juju plugin for running `kubectl` against the current model

use clap::{AppSettings, Clap};
use failure::Error;
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

#[derive(Clap)]
#[clap(about, author, version, setting(AppSettings::TrailingVarArg))]
struct Args {
    #[clap(short, long)]
    /// Model to operate in. Accepts [<controller name>:]<model name>
    model: Option<String>,

    #[clap(multiple = true)]
    /// Arguments to pass to kubectl
    commands: Vec<String>,
}

fn main() -> Result<(), Error> {
    let mut args: Args = Args::parse();
    let model_name = args.model.unwrap_or_else(String::new);
    let (controller_name, model_name) = parse_model_name(&model_name);

    let controllers = ControllerYaml::load()?;
    let models = ModelYaml::load()?;

    let controller_name = controllers.validate_name(controller_name)?;
    let model_name = models.validate_name(&controller_name, model_name)?;

    // Get all the extra args to pass onto `kubectl`
    let is_namespaced = args
        .commands
        .iter()
        .any(|c| c == "-n" || c == "-A" || c == "--all-namespaces" || c.starts_with("-n"));
    if !is_namespaced {
        args.commands.push("-n".to_string());
        args.commands.push(model_name);
    }

    let substrate = controllers.substrate(&controller_name)?;

    match substrate {
        Substrate::MicroK8s => {
            run("microk8s.kubectl", &args.commands)?;
        }
        Substrate::CDK => {
            let kubecfg = NamedTempFile::new()?;
            let path = &kubecfg.path().as_os_str().to_string_lossy();

            let kubectl_args = vec!["--kubeconfig".to_string(), path.to_string()]
                .into_iter()
                .chain(args.commands.into_iter())
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
            eprintln!("WARNING: Couldn't determine cloud substrate! Using default kubeconfig");
            run("kubectl", &args.commands)?;
        }
    }

    Ok(())
}
