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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::Reference;

    #[test]
    fn test_apply() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let tmp_path = tmp_dir.path().to_path_buf();
        let test_path = tmp_path.join("test");

        let setup = RepositorySetup {
            repository: "https://github.com/mrivnak/spaceconf.git".to_string(),
            reference: Reference::Branch("main".to_string()),
            path: test_path.clone(),
        };

        apply(setup);

        assert!(test_path.join(".git").exists());
    }

    #[test]
    #[should_panic]
    fn test_apply_existing() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let tmp_path = tmp_dir.path().to_path_buf();
        let test_path = tmp_path.join("test");

        let setup = RepositorySetup {
            repository: "https://github.com/mrivnak/spaceconf.git".to_string(),
            reference: Reference::Branch("main".to_string()),
            path: test_path.clone(),
        };
        std::fs::create_dir_all(&test_path).unwrap();

        apply(setup);
    }
}
