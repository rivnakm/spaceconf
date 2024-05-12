use crate::{fixture::RepositorySetup, git};

pub fn apply(setup: RepositorySetup) {
    if setup.path.exists() && !setup.path.join(".git").exists() {
        panic!("{} is not a git repository", setup.path.display());
    }

    if !setup.path.exists() {
        git::clone(&setup.repository, &setup.path, Some(setup.reference))
    }

    git::pull(&setup.path);
}
