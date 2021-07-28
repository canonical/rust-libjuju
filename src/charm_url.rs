use std::convert::TryFrom;
use std::path::PathBuf;
use std::str::FromStr;

use nom::bytes::complete::tag;
use nom::character::complete::{alpha1, digit1};
use nom::combinator::{map_res, opt};
use nom::sequence::{delimited, preceded, terminated, tuple};
use nom::{Err as NomErr, IResult, Needed};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_yaml::from_slice;

use crate::channel::Channel;
use crate::cmd::get_output;
use crate::error::JujuError;
use crate::store::Show;

/// Matches a `kebab-case` name that must not start or end with a dash
fn kebab_case(input: &str) -> IResult<&str, &str> {
    // Need some valid input
    if input.is_empty() {
        return Err(NomErr::Incomplete(Needed::Size(1)));
    }

    // Find the first invalid character, and split at it.
    // Validity is alphabetic and dash, unless it's at the start
    // or end of a string, in which case it's just alphabetic.
    let index = input
        .chars()
        .enumerate()
        .find(|&(i, ch)| {
            let alpha_next = input
                .chars()
                .nth(i + 1)
                .map(|ch| ch.is_alphabetic())
                .unwrap_or(false);

            let valid_char = ch.is_ascii_alphanumeric();
            let valid_dash = ch == '-' && i != 0 && alpha_next;

            !(valid_char || valid_dash)
        })
        .map(|(i, _)| i);

    // split_at returns `(match, remainder)`, and nom likes `(remainder, match)`.
    match index {
        Some(i) => {
            let split = input.split_at(i);
            Ok((split.1, split.0))
        }
        None => Ok(("", input)),
    }
}

/// Parses a charm store URL fragment
///
/// The only currently known one is `cs`, but in theory others are allowed
fn parse_store(input: &str) -> IResult<&str, Option<&str>> {
    opt(terminated(alpha1, tag(":")))(input)
}

/// Parses a namespace URL fragment
///
/// For example, `~foo-charmers/`
fn parse_namespace(input: &str) -> IResult<&str, Option<&str>> {
    opt(delimited(tag("~"), kebab_case, tag("/")))(input)
}

/// Parses a charm or bundle URL fragment
fn parse_name(input: &str) -> IResult<&str, &str> {
    kebab_case(input)
}

/// Parses a revision URL fragment
///
/// Must be zero or greater
fn parse_revision(input: &str) -> IResult<&str, Option<&str>> {
    opt(preceded(tag("-"), digit1))(input)
}

/// Parses a full charm store URL
fn parse_cs_url(input: &str) -> IResult<&str, CharmURL> {
    let joined = tuple((parse_store, parse_namespace, parse_name, parse_revision));

    map_res(joined, |(s, ns, n, r)| -> Result<CharmURL, String> {
        Ok(CharmURL {
            store: s.map(String::from),
            namespace: ns.map(String::from),
            name: n.to_string(),
            revision: r
                .map(|r| {
                    r.parse()
                        .map_err(|err| format!("Couldn't parse charm url revision: {}", err))
                })
                .transpose()?,
        })
    })(input)
}

/// Represents a charm's charm store URL
#[derive(Debug, PartialEq, Clone, Eq)]
pub struct CharmURL {
    pub store: Option<String>,
    pub namespace: Option<String>,
    pub name: String,
    pub revision: Option<u32>,
}

impl CharmURL {
    pub fn parse(input: &str) -> Result<Self, String> {
        let (remainder, url) = parse_cs_url(input)
            .map_err(|err| format!("Couldn't parse charm store url: {:?}", err))?;

        if !remainder.is_empty() {
            return Err(format!(
                "Got extra data at end of charm url string: `{}`",
                remainder
            ));
        }

        Ok(url)
    }

    /// Returns the un-namespaced charm name, for passing to the API of a particular charm store
    pub fn api_name(&self) -> String {
        let mut temp = self.clone();
        temp.store = None;
        temp.to_string()
    }

    pub fn from_path<P: Into<PathBuf>>(path: P) -> Self {
        CharmURL {
            store: None,
            namespace: None,
            name: path.into().to_string_lossy().to_string(),
            revision: None,
        }
    }

    pub fn with_namespace(&self, namespace: Option<String>) -> Self {
        CharmURL {
            namespace,
            ..self.clone()
        }
    }

    pub fn with_revision(&self, revision: Option<u32>) -> Self {
        CharmURL {
            revision,
            ..self.clone()
        }
    }

    pub fn show(&self, channel: Channel) -> Result<Show, JujuError> {
        let output = get_output(
            "charm",
            &[
                "show",
                &self.to_string(),
                "--channel",
                &channel.to_string(),
                "--format",
                "yaml",
            ],
        )?;
        Ok(from_slice(&output)?)
    }
}

impl FromStr for CharmURL {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl TryFrom<&str> for CharmURL {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl ToString for CharmURL {
    fn to_string(&self) -> String {
        let mut serialized = String::new();

        if let Some(st) = &self.store {
            serialized.push_str(&format!("{}:", st))
        }

        if let Some(ns) = &self.namespace {
            serialized.push_str(&format!("~{}/", ns))
        }

        serialized.push_str(&self.name);

        if let Some(rev) = &self.revision {
            serialized.push_str(&format!("-{}", rev))
        }

        serialized
    }
}

impl Serialize for CharmURL {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for CharmURL {
    fn deserialize<D>(deserializer: D) -> Result<CharmURL, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        let s = String::deserialize(deserializer)?;

        s.parse()
            .map_err(|err| Error::custom(format!("Error deserializing CharmURL: {}", err)))
    }
}

#[cfg(test)]
mod tests {
    use serde_yaml::{from_str, to_string};

    use super::*;

    #[test]
    fn test_alphadash() {
        let (remainder, parsed) = kebab_case("foo-foo").unwrap();

        assert_eq!(parsed, "foo-foo");
        assert_eq!(remainder, "");
    }

    #[test]
    fn test_store() {
        let (remainder, parsed) = parse_store("cs:~foo/bar-42").unwrap();

        assert_eq!(parsed, Some("cs"));
        assert_eq!(remainder, "~foo/bar-42");
    }

    #[test]
    fn test_namespace() {
        let (remainder, parsed) = parse_namespace("~foo/bar-42").unwrap();

        assert_eq!(parsed, Some("foo"));
        assert_eq!(remainder, "bar-42");
    }

    #[test]
    fn test_name() {
        let (remainder, parsed) = parse_name("bar-42").unwrap();

        assert_eq!(parsed, "bar");
        assert_eq!(remainder, "-42");

        let (remainder, parsed) = parse_name("k8s-42").unwrap();

        assert_eq!(parsed, "k8s");
        assert_eq!(remainder, "-42");

        let (remainder, parsed) = parse_name("bar-").unwrap();
        assert_eq!(parsed, "bar");
        assert_eq!(remainder, "-");

        let (remainder, parsed) = parse_name("-bar").unwrap();
        assert_eq!(parsed, "");
        assert_eq!(remainder, "-bar");
    }

    #[test]
    fn test_revision() {
        let (remainder, parsed) = parse_revision("-42").unwrap();

        assert_eq!(parsed, Some("42"));
        assert_eq!(remainder, "");
    }

    #[test]
    fn test_full_valid() {
        let charm_url: CharmURL = "cs:~foo/bar-42".parse().unwrap();

        assert_eq!(
            charm_url,
            CharmURL {
                store: Some("cs".to_string()),
                namespace: Some("foo".to_string()),
                name: "bar".to_string(),
                revision: Some(42),
            }
        );
    }

    #[test]
    fn test_full_bad_rev() {
        let charm_url: Result<CharmURL, String> = "cs:~foo/bar-4294967296".parse();

        assert!(charm_url.is_err());
    }

    #[test]
    fn test_partial() {
        let urls = vec![
            "cs:~foo-foo/bar-42",
            "cs:~foo/bar",
            "cs:bar-42",
            "cs:bar",
            "~foo/bar-42",
            "~foo/bar",
            "bar-42",
            "bar",
        ];

        let expecteds = vec![
            CharmURL {
                store: Some("cs".to_string()),
                namespace: Some("foo-foo".to_string()),
                name: "bar".to_string(),
                revision: Some(42),
            },
            CharmURL {
                store: Some("cs".to_string()),
                namespace: Some("foo".to_string()),
                name: "bar".to_string(),
                revision: None,
            },
            CharmURL {
                store: Some("cs".to_string()),
                namespace: None,
                name: "bar".to_string(),
                revision: Some(42),
            },
            CharmURL {
                store: Some("cs".to_string()),
                namespace: None,
                name: "bar".to_string(),
                revision: None,
            },
            CharmURL {
                store: None,
                namespace: Some("foo".to_string()),
                name: "bar".to_string(),
                revision: Some(42),
            },
            CharmURL {
                store: None,
                namespace: Some("foo".to_string()),
                name: "bar".to_string(),
                revision: None,
            },
            CharmURL {
                store: None,
                namespace: None,
                name: "bar".to_string(),
                revision: Some(42),
            },
            CharmURL {
                store: None,
                namespace: None,
                name: "bar".to_string(),
                revision: None,
            },
        ];

        for (url, expected) in urls.into_iter().zip(expecteds) {
            println!("Testing {}", url);
            let parsed: CharmURL = url.parse().unwrap();
            assert_eq!(parsed, expected);
        }
    }

    #[test]
    fn test_serialization() {
        let charm_url = CharmURL {
            store: Some("cs".into()),
            namespace: Some("foo-foo".to_string()),
            name: "bar-bar".to_string(),
            revision: Some(42),
        };

        let serialized = "---\n\"cs:~foo-foo/bar-bar-42\"";

        assert_eq!(&to_string(&charm_url).unwrap()[..], serialized);

        let parsed: CharmURL = from_str(serialized).unwrap();
        assert_eq!(parsed, charm_url);
    }
}
