use std::collections::HashMap;
use std::convert::TryInto;
use std::fs;
use std::path::PathBuf;

use juju::bundle::{Annotations, Application, Bundle};
use juju::local::controller::{Controller, KubernetesPortForwardConfig, ProxyConfig};
use juju::local::ControllerYaml;
use juju::series::Series;

#[test]
fn parse_bundle() {
    let bundle = Bundle::load(PathBuf::from("tests/examples/bundle-basic.yaml")).unwrap();
    let mut applications = HashMap::new();

    applications.insert(
        "foo".to_string(),
        Application {
            annotations: Some(Annotations {
                gui_x: "0".into(),
                gui_y: "0".into(),
            }),
            source: Some("./foo".into()),
            charm: Some("cs:foo".try_into().unwrap()),
            scale: 1,
            ..Default::default()
        },
    );

    applications.insert(
        "bar".to_string(),
        Application {
            charm: Some("cs:bar".try_into().unwrap()),
            scale: 1,
            ..Default::default()
        },
    );

    assert_eq!(
        bundle,
        Bundle {
            applications,
            description: Some("An awesome bundle".to_string()),
            relations: vec![vec!["foo", "bar"]
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>()],
            bundle: Some(Series::Kubernetes),
            series: None,
        }
    );
}

#[test]
fn parse_controller_yaml() {
    let bytes = fs::read(PathBuf::from("tests/examples/controllers.yaml")).unwrap();
    let parsed = ControllerYaml::load_from_bytes(&bytes).unwrap();

    let mut dns_cache = HashMap::new();
    dns_cache.insert("localhost".into(), vec!["::1".into(), "127.0.0.1".into()]);

    let mut controllers = HashMap::new();
    controllers.insert(
        "uk8s".into(),
        Controller {
            active_controller_machine_count: 0,
            agent_version: "2.9-rc5".into(),
            api_endpoints: vec!["localhost:1234".into()],
            ca_cert: "CERTIFICATE".into(),
            cloud: "microk8s".into(),
            controller_machine_count: 1,
            dns_cache,
            kind: Some("kubernetes".into()),
            machine_count: 1,
            proxy_config: Some(ProxyConfig::KubernetesPortForward {
                config: KubernetesPortForwardConfig {
                    api_host: "https://10.0.0.1:16443".into(),
                    ca_cert: "CERTIFICATE".into(),
                    namespace: "controller-uk8s".into(),
                    remote_port: "17070".into(),
                    service: "controller-service".into(),
                    service_account_token: "TOKEN".into(),
                },
            }),
            region: Some("localhost".into()),
            uuid: "d9df0ce4-caec-4c65-8a86-389627ac9845".into(),
        },
    );

    let expected = ControllerYaml {
        current_controller: Some("uk8s".into()),
        controllers,
    };

    assert_eq!(parsed, expected);
}
