use std::path::PathBuf;

use log::error;

use crate::fixture::{Fixture, FixtureType};

pub fn load_fixtures(dir: PathBuf, names: Vec<String>) -> std::io::Result<Vec<Fixture>> {
    let dir_entries = std::fs::read_dir(dir)?;
    let fixture_dirs = dir_entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
        .filter(|entry| {
            let fixure_dir_entries = std::fs::read_dir(entry.path()).unwrap();
            fixure_dir_entries
                .filter_map(|entry| entry.ok())
                .any(|entry| entry.file_name() == "fixture.json")
        })
        .map(|entry| entry.path())
        .filter(|path| {
            if names.is_empty() {
                return true;
            }

            let fixture_name = path.file_name().unwrap().to_string_lossy();
            names.contains(&fixture_name.to_string())
        });

    let fixtures = fixture_dirs
        .map(|fixture_dir| {
            let fixture_file = fixture_dir.join("fixture.json");
            let secret_file = fixture_dir.join("secrets.json");
            let fixture = std::fs::read_to_string(fixture_file).unwrap();
            let mut fixture: Fixture = serde_json::from_str(&fixture).unwrap();

            // resolve relative paths to absolute paths and load secrets
            if let FixtureType::Files(ref mut setup) = &mut fixture.fixture_type {
                for file in &mut setup.files {
                    file.src = file.src.clone().expand(&fixture_dir);
                }
                if secret_file.exists() {
                    let secrets = std::fs::read_to_string(secret_file).unwrap();
                    setup.secrets = serde_json::from_str(&secrets).unwrap();
                }
            }

            if fixture.name.is_empty() {
                fixture.name = fixture_dir
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
            }

            let _ = fixture.validate().inspect_err(|e| {
                error!("Invalid fixture: {}", e);
                std::process::exit(1);
            });

            fixture
        })
        .collect();

    // TODO: process fixtures
    // assign the name if it's not present
    // validate the fixture

    Ok(fixtures)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::fixture::{File, FileDefinition, FilesSetup};

    use super::*;

    #[test]
    fn test_load_fixtures() {
        let test_dir = tempfile::tempdir().expect("Failed to create temporary directory");

        let fixture_dir = test_dir.path().join("test-fixture");
        std::fs::create_dir(&fixture_dir).unwrap();

        let fixture_file = fixture_dir.join("fixture.json");
        let fixture = Fixture {
            name: "test-fixture".into(),
            include_for: None,
            exclude_for: None,
            fixture_type: FixtureType::Files(FilesSetup {
                files: vec![File {
                    src: FileDefinition::Single("source.conf".into()),
                    dest: FileDefinition::Single("/etc/dest.conf".into()),
                    raw: false,
                    optional: false,
                }],
                root: false,
                secrets: Default::default(),
            }),
        };
        std::fs::write(fixture_file, serde_json::to_string(&fixture).unwrap()).unwrap();

        let fixtures = load_fixtures(test_dir.path().to_path_buf(), vec![]).unwrap();

        assert_eq!(fixtures.len(), 1);
        assert!(matches!(fixtures[0].fixture_type, FixtureType::Files(_)));

        assert_eq!(fixtures[0].name, "test-fixture".to_string());

        let setup = match &fixtures[0].fixture_type {
            FixtureType::Files(setup) => setup,
            _ => unreachable!(),
        };

        assert_eq!(setup.files.len(), 1);
        assert_eq!(
            setup.files[0].src,
            FileDefinition::Single(fixture_dir.join("source.conf"))
        );
        assert_eq!(
            setup.files[0].dest,
            FileDefinition::Single(PathBuf::from("/etc/dest.conf"))
        );
        assert!(!setup.files[0].raw);
        assert!(!setup.files[0].optional);

        assert!(!setup.root);
        assert!(setup.secrets.is_empty());
    }

    #[test]
    fn test_get_fixtures_by_name() {
        let test_dir = tempfile::tempdir().expect("Failed to create temporary directory");

        let fixture_dir = test_dir.path().join("test-fixture");
        std::fs::create_dir(&fixture_dir).unwrap();

        let fixture_file = fixture_dir.join("fixture.json");
        let fixture = Fixture {
            name: "test-fixture".into(),
            include_for: None,
            exclude_for: None,
            fixture_type: FixtureType::Files(FilesSetup {
                files: vec![File {
                    src: FileDefinition::Single("source.conf".into()),
                    dest: FileDefinition::Single("/etc/dest.conf".into()),
                    raw: false,
                    optional: false,
                }],
                root: false,
                secrets: Default::default(),
            }),
        };
        std::fs::write(fixture_file, serde_json::to_string(&fixture).unwrap()).unwrap();
        let fixtures =
            load_fixtures(test_dir.path().to_path_buf(), vec!["test-fixture".into()]).unwrap();

        assert_eq!(fixtures.len(), 1);
        assert!(matches!(fixtures[0].fixture_type, FixtureType::Files(_)));

        assert_eq!(fixtures[0].name, "test-fixture".to_string());

        let setup = match &fixtures[0].fixture_type {
            FixtureType::Files(setup) => setup,
            _ => unreachable!(),
        };

        assert_eq!(setup.files.len(), 1);
        assert_eq!(
            setup.files[0].src,
            FileDefinition::Single(fixture_dir.join("source.conf"))
        );
        assert_eq!(
            setup.files[0].dest,
            FileDefinition::Single(PathBuf::from("/etc/dest.conf"))
        );
        assert!(!setup.files[0].raw);
        assert!(!setup.files[0].optional);

        assert!(!setup.root);
        assert!(setup.secrets.is_empty());
    }

    #[test]
    fn test_get_fixtures_with_secrets() {
        let test_dir = tempfile::tempdir().expect("Failed to create temporary directory");

        let fixture_dir = test_dir.path().join("test-fixture");
        std::fs::create_dir(&fixture_dir).unwrap();

        let fixture_file = fixture_dir.join("fixture.json");
        let fixture = Fixture {
            name: "test-fixture".into(),
            include_for: None,
            exclude_for: None,
            fixture_type: FixtureType::Files(FilesSetup {
                files: vec![File {
                    src: FileDefinition::Single("source.conf".into()),
                    dest: FileDefinition::Single("/etc/dest.conf".into()),
                    raw: false,
                    optional: false,
                }],
                root: false,
                secrets: Default::default(),
            }),
        };
        std::fs::write(fixture_file, serde_json::to_string(&fixture).unwrap()).unwrap();

        let secrets_file = fixture_dir.join("secrets.json");
        let secrets: HashMap<String, String> =
            HashMap::from_iter(vec![("key".into(), "value".into())]);
        std::fs::write(secrets_file, serde_json::to_string(&secrets).unwrap()).unwrap();

        let fixtures = load_fixtures(test_dir.path().to_path_buf(), vec![]).unwrap();

        assert_eq!(fixtures.len(), 1);
        assert!(matches!(fixtures[0].fixture_type, FixtureType::Files(_)));

        assert_eq!(fixtures[0].name, "test-fixture".to_string());

        let setup = match &fixtures[0].fixture_type {
            FixtureType::Files(setup) => setup,
            _ => unreachable!(),
        };

        assert_eq!(setup.files.len(), 1);
        assert_eq!(
            setup.files[0].src,
            FileDefinition::Single(fixture_dir.join("source.conf"))
        );
        assert_eq!(
            setup.files[0].dest,
            FileDefinition::Single(PathBuf::from("/etc/dest.conf"))
        );
        assert!(!setup.files[0].raw);
        assert!(!setup.files[0].optional);

        assert!(!setup.root);
        assert_eq!(setup.secrets.get("key"), Some(&"value".into()));
    }

    #[test]
    fn test_get_fixtures_empty_name() {
        let test_dir = tempfile::tempdir().expect("Failed to create temporary directory");

        let fixture_dir = test_dir.path().join("test-fixture");
        std::fs::create_dir(&fixture_dir).unwrap();

        let fixture_file = fixture_dir.join("fixture.json");
        let fixture = Fixture {
            name: "".into(),
            include_for: None,
            exclude_for: None,
            fixture_type: FixtureType::Files(FilesSetup {
                files: vec![File {
                    src: FileDefinition::Single("source.conf".into()),
                    dest: FileDefinition::Single("/etc/dest.conf".into()),
                    raw: false,
                    optional: false,
                }],
                root: false,
                secrets: Default::default(),
            }),
        };
        std::fs::write(fixture_file, serde_json::to_string(&fixture).unwrap()).unwrap();

        let fixtures = load_fixtures(test_dir.path().to_path_buf(), vec![]).unwrap();

        assert_eq!(fixtures[0].name, "test-fixture".to_string());

        assert_eq!(fixtures.len(), 1);
        assert!(matches!(fixtures[0].fixture_type, FixtureType::Files(_)));

        let setup = match &fixtures[0].fixture_type {
            FixtureType::Files(setup) => setup,
            _ => unreachable!(),
        };

        assert_eq!(setup.files.len(), 1);
        assert_eq!(
            setup.files[0].src,
            FileDefinition::Single(fixture_dir.join("source.conf"))
        );
        assert_eq!(
            setup.files[0].dest,
            FileDefinition::Single(PathBuf::from("/etc/dest.conf"))
        );
        assert!(!setup.files[0].raw);
        assert!(!setup.files[0].optional);

        assert!(!setup.root);
        assert!(setup.secrets.is_empty());
    }
}
