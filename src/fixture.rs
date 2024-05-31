use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

type Specifier = String;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Fixture {
    #[serde(default)]
    pub name: String,
    pub(crate) include_for: Option<Vec<Specifier>>,
    pub(crate) exclude_for: Option<Vec<Specifier>>,

    #[serde(flatten)]
    pub fixture_type: FixtureType,
}

impl Fixture {
    pub fn validate(&self) -> Result<(), String> {
        match &self.fixture_type {
            FixtureType::Files(files) => {
                if files.files.is_empty() {
                    return Err("Files fixture must have at least one file".to_string());
                }

                for file in &files.files {
                    if file.src.clone().resolve().is_none() && !file.optional {
                        return Err(
                            "Source file cannot be resolved and is not marked as optional"
                                .to_string(),
                        );
                    }

                    if file.dest.clone().resolve().is_none() && !file.optional {
                        return Err(
                            "Destination file cannot be resolved and is not marked as optional"
                                .to_string(),
                        );
                    }
                }
            }
            FixtureType::Repository(repo) => {
                if repo.repository.is_empty() {
                    return Err("Repository fixture must have a repository URL".to_string());
                }
            }
        }

        Ok(())
    }

    pub fn skip(&self) -> bool {
        todo!();
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type")]
pub enum FixtureType {
    Files(FilesSetup),
    Repository(RepositorySetup),
}

impl Default for FixtureType {
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
    pub src: FileDefinition,
    pub dest: FileDefinition,

    #[serde(default)]
    pub raw: bool,

    #[serde(default)]
    pub optional: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum FileDefinition {
    Single(PathBuf),
    Multiple(HashMap<Specifier, PathBuf>),
}

impl FileDefinition {
    pub fn resolve(self) -> Option<PathBuf> {
        match self {
            FileDefinition::Single(path) => Some(path),
            FileDefinition::Multiple(map) => {
                todo!();
            }
        }
    }

    pub fn expand(self, fixture_path: &Path) -> Self {
        match self {
            FileDefinition::Single(path) => FileDefinition::Single(fixture_path.join(path)),
            FileDefinition::Multiple(map) => {
                let mut new_map = HashMap::new();
                for (key, value) in map {
                    new_map.insert(key, fixture_path.join(value));
                }

                FileDefinition::Multiple(new_map)
            }
        }
    }
}

fn choose_spec(specs: &[Specifier]) -> Option<Specifier> {
    use std::env::consts::{ARCH, OS};
    if let Ok(hostname) = hostname::get() {
        if let Some(hostname) = hostname.to_string_lossy().split('.').next() {
            // Hostname exact match
            if specs.contains(&hostname.to_string()) {
                return Some(hostname.to_string());
            }

            // Hostname glob match
            for spec in specs {
                let glob = globset::Glob::new(spec)
                    .expect("Invalid glob")
                    .compile_matcher();

                if glob.is_match(hostname) {
                    return Some(spec.clone());
                }
            }
        }
    }

    // OS-ARCH match
    if specs.contains(&format!("{}-{}", OS, ARCH)) {
        return Some(format!("{}-{}", OS, ARCH));
    }

    // OS match
    if specs.contains(&OS.to_string()) {
        return Some(OS.to_string());
    }

    // Default case
    if specs.contains(&"default".to_string()) {
        return Some("default".to_string());
    }

    None
}

fn matches_spec(specs: &[Specifier]) -> bool {
    choose_spec(specs).is_some()
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
                src: FileDefinition::Single(PathBuf::from("src")),
                dest: FileDefinition::Single(PathBuf::from("dest")),
                raw: false,
                optional: false,
            }],
            root: true,
            secrets: HashMap::new(),
        };

        let actual: FilesSetup = serde_json::from_str(input).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_deserialize_files_setup_multiple() {
        let input = r#"{
            "type": "files",
            "files": [
                {
                    "src": {
                        "windows": "src/windows",
                        "default": "src/default"
                    },
                    "dest": "dest"
                }
            ],
            "root": true
        }"#;

        let expected = FilesSetup {
            files: vec![File {
                src: FileDefinition::Multiple({
                    let mut map = HashMap::new();
                    map.insert("windows".to_string(), PathBuf::from("src/windows"));
                    map.insert("default".to_string(), PathBuf::from("src/default"));
                    map
                }),
                dest: FileDefinition::Single(PathBuf::from("dest")),
                raw: false,
                optional: false,
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

    #[test]
    fn test_choose_spec_os() {
        let specs = vec![
            "windows".to_string(),
            "macos".to_string(),
            "linux".to_string(),
        ];

        assert_eq!(choose_spec(&specs), Some(std::env::consts::OS.to_string()));
    }

    #[test]
    fn test_choose_spec_os_no_match() {
        let specs = vec![
            "ios".to_string(),
            "android".to_string(),
            "solaris".to_string(),
        ];

        assert_eq!(choose_spec(&specs), None);
    }

    #[test]
    fn test_choose_spec_os_no_match_default() {
        let specs = vec![
            "ios".to_string(),
            "android".to_string(),
            "default".to_string(),
        ];

        assert_eq!(choose_spec(&specs), Some("default".to_string()));
    }

    #[test]
    fn test_choose_spec_os_arch() {
        use std::env::consts::{ARCH, OS};
        let specs = vec![
            format!("{}-x86_64", OS),
            format!("{}-aarch64", OS),
            format!("{}-arm", OS),
        ];

        assert_eq!(choose_spec(&specs), Some(format!("{}-{}", OS, ARCH)));
    }

    #[test]
    fn test_choose_spec_hostname() {
        let hostname = hostname::get().unwrap().to_string_lossy().to_string();
        let specs = vec![
            "windows".to_string(),
            "macos".to_string(),
            "linux".to_string(),
            hostname.clone(),
        ];

        assert_eq!(choose_spec(&specs), Some(hostname));
    }

    #[test]
    fn test_choose_spec_hostname_glob() {
        let mut hostname = hostname::get().unwrap().to_string_lossy().to_string();
        hostname.truncate(hostname.len() - 2);
        let glob = hostname + "*";

        let specs = vec![
            "windows".to_string(),
            "macos".to_string(),
            "linux".to_string(),
            glob.clone(),
        ];

        assert_eq!(choose_spec(&specs), Some(glob));
    }

    #[test]
    fn test_choose_spec_hostname_glob_prefers_exact() {
        let hostname = hostname::get().unwrap().to_string_lossy().to_string();
        let mut truncated = hostname.clone();
        truncated.truncate(truncated.len() - 2);
        let glob = truncated + "*";

        let specs = vec![
            "windows".to_string(),
            "macos".to_string(),
            "linux".to_string(),
            glob,
            hostname.clone(),
        ];

        assert_eq!(choose_spec(&specs), Some(hostname));
    }
}
