use std::collections::HashMap;
use std::convert::TryInto;
use std::fs;
use std::path::PathBuf;

use serde_yaml::from_slice;

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
            name: None,
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

#[test]
fn parse_metadata_yaml() {
    use juju::charm_source as cs;

    let bytes = fs::read(PathBuf::from("tests/examples/kubernetes-metadata.yaml")).unwrap();
    let parsed: cs::Metadata = from_slice(&bytes).unwrap();

    let containers = [
        (
            "super-app".into(),
            cs::Container::Resource(cs::ResourceContainer {
                resource: "super-app-image".into(),
                mounts: vec![cs::ContainerMount {
                    storage: "logs".into(),
                    location: "/logs".into(),
                }],
            }),
        ),
        (
            "super-app-helper".into(),
            cs::Container::Base(cs::BaseContainer {
                bases: vec![cs::ContainerBase {
                    name: "ubuntu".into(),
                    channel: "ubuntu/20.04".into(),
                    architectures: vec!["amd64".into(), "arm64".into()],
                }],
                mounts: vec![],
            }),
        ),
    ]
    .into();

    let resources = [
        (
            "super-app-image".into(),
            cs::Resource::OciImage {
                description: Some(
                    "OCI image for the Super App (hub.docker.com/_/super-app)".into(),
                ),
                upstream_source: None,
            },
        ),
        (
            "definitions".into(),
            cs::Resource::File {
                description: Some(
                    "A small SQLite3 database of definitions needed by super app".into(),
                ),
                filename: "definitions.db".into(),
            },
        ),
    ]
    .into();

    let provides = [(
        "super-worker".into(),
        cs::Relation {
            interface: "super-worker".into(),
            ..Default::default()
        },
    )]
    .into();

    let requires = [(
        "ingress".into(),
        cs::Relation {
            interface: "ingress".into(),
            limit: Some(1),
            optional: true,
            ..Default::default()
        },
    )]
    .into();

    let peer = [(
        "super-replicas".into(),
        cs::Relation {
            interface: "super-replicas".into(),
            ..Default::default()
        },
    )]
    .into();

    let storage = [(
        "logs".into(),
        cs::Storage::Filesystem {
            description: Some("Storage mount for application logs".into()),
            location: Some("/logs".into()),
            shared: true,
            read_only: false,
            multiple: None,
            minimum_size: None,
            properties: vec![],
        },
    )]
    .into();

    let expected = cs::Metadata {
        name: "super-charm".into(),
        summary: "a really great charm".into(),
        description: "This is a really great charm, whose metadata is suitably complete so as to\ndemonstrate as many of the fields as possible.\n".into(),
        maintainers: vec!["Joe Bloggs <joe.bloggs@email.com>".into()],
        terms: vec![],
        subordinate: false,
        containers,
        resources,
        provides,
        requires,
        peer,
        storage,
        devices: HashMap::new(),
        extra_bindings: HashMap::new(),
        series: None,
    };

    assert_eq!(parsed, expected);
}
