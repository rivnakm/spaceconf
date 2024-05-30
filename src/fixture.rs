use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type")]
pub enum Fixture {
    Files(FilesSetup),
    Repository(RepositorySetup),
}

impl Default for Fixture {
    fn default() -> Self {
        Self::Files(Default::default())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct FilesSetup {
    #[serde(default)]
    pub files: Vec<File>,

    #[serde(default)]
    pub root: bool,

    #[serde(skip)]
    pub secrets: HashMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct File {
    pub src: PathBuf,
    pub dest: PathBuf,

    #[serde(default)]
    pub raw: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct RepositorySetup {
    pub repository: String,
    pub reference: Reference,
    pub path: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type", content = "value")]
pub enum Reference {
    Branch(String),
    Tag(String),
    Commit(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_files_setup() {
        let input = r#"{
            "type": "files",
            "files": [
                {
                    "src": "src",
                    "dest": "dest"
                }
            ],
            "root": true
        }"#;

        let expected = FilesSetup {
            files: vec![File {
                src: PathBuf::from("src"),
                dest: PathBuf::from("dest"),
                raw: false,
            }],
            root: true,
            secrets: HashMap::new(),
        };

        let actual: FilesSetup = serde_json::from_str(input).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_deserialize_repository_setup() {
        let input = r#"{
            "type": "repository",
            "repository": "https://github.com/torvals/linux.git",
            "reference": {
                "type": "branch",
                "value": "master"
            },
            "path": "linux"
        }"#;

        let expected = RepositorySetup {
            repository: "https://github.com/torvals/linux.git".to_string(),
            reference: Reference::Branch("master".to_string()),
            path: PathBuf::from("linux"),
        };

        let actual: RepositorySetup = serde_json::from_str(input).unwrap();
        assert_eq!(actual, expected);
    }
}
