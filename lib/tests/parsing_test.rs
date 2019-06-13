use std::collections::HashMap;
use std::path::PathBuf;

use juju::parsing::bundle::{Annotations, Application, Bundle};
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
            charm: Some("cs:foo".into()),
            scale: 1,
            ..Default::default()
        },
    );

    applications.insert(
        "bar".to_string(),
        Application {
            charm: Some("cs:bar".into()),
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
