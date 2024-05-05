use std::path::PathBuf;

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

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct FilesSetup {
    #[serde(default)]
    pub files: Vec<File>,

    #[serde(default)]
    pub root: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct File {
    pub src: PathBuf,
    pub dest: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RepositorySetup {
    pub repository: String,
    pub reference: Reference,
    pub path: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Reference {
    Branch(String),
    Tag(String),
    Commit(String),
}
