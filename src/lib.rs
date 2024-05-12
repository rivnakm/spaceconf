use std::path::PathBuf;

use fixture::Fixture;
use resolve_path::PathResolveExt;

mod fixture;
pub mod git;
mod repo;
mod template;

fn get_fixtures(dir: PathBuf) -> std::io::Result<Vec<Fixture>> {
    let dir_entries = std::fs::read_dir(dir)?;
    let fixture_dirs: Vec<PathBuf> = dir_entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
        .filter(|entry| {
            let fixure_dir_entries = std::fs::read_dir(entry.path()).unwrap();
            fixure_dir_entries
                .filter_map(|entry| entry.ok())
                .any(|entry| entry.file_name() == "fixture.json")
        })
        .map(|entry| entry.path())
        .collect();

    let fixtures = fixture_dirs
        .into_iter()
        .map(|fixture_dir| {
            let fixture_file = fixture_dir.join("fixture.json");
            let secret_file = fixture_dir.join("secrets.json");
            let fixture = std::fs::read_to_string(fixture_file).unwrap();
            let mut fixture: Fixture = serde_json::from_str(&fixture).unwrap();
            match &mut fixture {
                fixture::Fixture::Files(ref mut setup) => {
                    for file in &mut setup.files {
                        file.src = fixture_dir.join(&file.src);
                        file.dest = file.dest.resolve().to_path_buf();
                    }
                    if secret_file.exists() {
                        let secrets = std::fs::read_to_string(secret_file).unwrap();
                        setup.secrets = serde_json::from_str(&secrets).unwrap();
                    }
                }
                fixture::Fixture::Repository(ref mut setup) => {
                    setup.path = setup.path.resolve().to_path_buf();
                }
            }
            fixture
        })
        .collect();

    Ok(fixtures)
}

pub fn load_fixtures(dir: PathBuf, root: bool) -> std::io::Result<Vec<Fixture>> {
    let fixtures = get_fixtures(dir)?;

    Ok(fixtures
        .into_iter()
        .filter(|fixture| match &fixture {
            fixture::Fixture::Files(setup) => setup.root == root,
            fixture::Fixture::Repository(_) => !root,
        })
        .collect())
}

pub fn list_fixtures(fixtures: Vec<Fixture>) {
    for fixture in fixtures {
        match &fixture {
            fixture::Fixture::Files(setup) => {
                println!(
                    "Fixture: {}",
                    setup.files[0].src.parent().unwrap().display()
                );
                for file in &setup.files {
                    println!("  File: {}", file.dest.display());
                }
            }
            fixture::Fixture::Repository(setup) => {
                println!("Fixture: {}", setup.repository);
                println!("  Reference: {:?}", setup.reference);
                println!("  Path: {}", setup.path.display());
            }
        }
    }
}

pub fn check_fixtures(fixtures: Vec<Fixture>) {
    for fixture in fixtures {
        match &fixture {
            fixture::Fixture::Files(setup) => {
                for file in &setup.files {
                    if !file.dest.exists() {
                        println!("{:?} does not exist", file.dest);
                        continue;
                    }

                    let src_time = file.src.metadata().unwrap().modified().unwrap();
                    let dest_time = file.dest.metadata().unwrap().modified().unwrap();

                    if src_time <= dest_time {
                        println!("{:?} is up to date", file.dest);
                    } else {
                        println!("{:?} is NOT up to date", file.dest);
                    }
                }
            }
            fixture::Fixture::Repository(_) => {
                unimplemented!("'check' command is not supported for repository fixtures");
            }
        }
    }
}

pub fn apply_fixtures(fixtures: Vec<Fixture>, revert: bool) {
    for fixture in fixtures {
        match &fixture {
            fixture::Fixture::Files(setup) => {
                for file in &setup.files {
                    if revert {
                        unimplemented!("store the original file content and restore it here");
                    } else if file.raw {
                        println!("Applying {:?}", file.dest);
                        std::fs::copy(&file.src, &file.dest).unwrap();
                    } else {
                        let input = std::fs::read_to_string(&file.src).unwrap();
                        let output = template::render(&input, &setup.secrets).unwrap();

                        println!("Applying {:?}", file.dest);
                        // TODO: make backup of the original file
                        std::fs::write(&file.dest, output).unwrap();
                    }
                }
            }
            fixture::Fixture::Repository(setup) => {
                repo::apply(setup.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::{self, Fixture};

    #[test]
    fn test_get_fixtures() {
        let test_dir = tempfile::tempdir().expect("Failed to create temporary directory");

        let fixture_dir = test_dir.path().join("test-fixture");
        std::fs::create_dir(&fixture_dir).unwrap();

        let fixture_file = fixture_dir.join("fixture.json");
        let fixture = Fixture::Files(fixture::FilesSetup {
            files: vec![fixture::File {
                src: "source.conf".into(),
                dest: "/etc/dest.conf".into(),
                raw: false,
            }],
            root: false,
            secrets: Default::default(),
        });
        std::fs::write(fixture_file, serde_json::to_string(&fixture).unwrap()).unwrap();

        let fixtures = get_fixtures(test_dir.path().to_path_buf()).unwrap();

        assert_eq!(fixtures.len(), 1);
        assert!(matches!(fixtures[0], Fixture::Files(_)));

        let setup = match &fixtures[0] {
            Fixture::Files(setup) => setup,
            _ => unreachable!(),
        };

        assert_eq!(setup.files.len(), 1);
        assert_eq!(setup.files[0].src, fixture_dir.join("source.conf"));
        assert_eq!(setup.files[0].dest, PathBuf::from("/etc/dest.conf"));
        assert!(!setup.files[0].raw);

        assert!(!setup.root);
        assert!(setup.secrets.is_empty());
    }

    #[test]
    fn test_load_fixtures() {
        let test_dir = tempfile::tempdir().expect("Failed to create temporary directory");

        let fixture_dir1 = test_dir.path().join("test-fixture1");
        let fixture_dir2 = test_dir.path().join("test-fixture2");
        let fixture_dir3 = test_dir.path().join("test-fixture3");
        std::fs::create_dir(&fixture_dir1).unwrap();
        std::fs::create_dir(&fixture_dir2).unwrap();
        std::fs::create_dir(&fixture_dir3).unwrap();

        let fixture1_file = fixture_dir1.join("fixture.json");
        let fixture1 = Fixture::Files(fixture::FilesSetup {
            files: vec![fixture::File {
                src: "source.conf".into(),
                dest: "/etc/dest.conf".into(),
                raw: false,
            }],
            root: false,
            secrets: Default::default(),
        });

        let fixture2_file = fixture_dir2.join("fixture.json");
        let fixture2 = Fixture::Files(fixture::FilesSetup {
            files: vec![fixture::File {
                src: "source.conf".into(),
                dest: "/etc/dest.conf".into(),
                raw: false,
            }],
            root: true,
            secrets: Default::default(),
        });

        let fixture3_file = fixture_dir3.join("fixture.json");
        let fixture3 = Fixture::Repository(fixture::RepositorySetup {
            repository: "https://github.com/user/repo".into(),
            reference: fixture::Reference::Branch("main".into()),
            path: "/etc/repo".into(),
        });
        std::fs::write(fixture1_file, serde_json::to_string(&fixture1).unwrap()).unwrap();
        std::fs::write(fixture2_file, serde_json::to_string(&fixture2).unwrap()).unwrap();
        std::fs::write(fixture3_file, serde_json::to_string(&fixture3).unwrap()).unwrap();

        let fixtures = load_fixtures(test_dir.path().to_path_buf(), false).unwrap();

        assert_eq!(fixtures.len(), 2);
        assert!(matches!(fixtures[0], Fixture::Files(_)));
        assert!(matches!(fixtures[1], Fixture::Repository(_)));

        let setup = match &fixtures[0] {
            Fixture::Files(setup) => setup,
            _ => unreachable!(),
        };

        assert_eq!(setup.files.len(), 1);
        assert_eq!(setup.files[0].src, fixture_dir1.join("source.conf"));
        assert_eq!(setup.files[0].dest, PathBuf::from("/etc/dest.conf"));
        assert!(!setup.files[0].raw);

        assert!(!setup.root);
        assert!(setup.secrets.is_empty());

        let setup = match &fixtures[1] {
            Fixture::Repository(setup) => setup,
            _ => unreachable!(),
        };

        assert_eq!(setup.repository, "https://github.com/user/repo");
        assert_eq!(setup.reference, fixture::Reference::Branch("main".into()));
        assert_eq!(setup.path, PathBuf::from("/etc/repo"));

        let fixtures = load_fixtures(test_dir.path().to_path_buf(), true).unwrap();

        assert_eq!(fixtures.len(), 1);
        assert!(matches!(fixtures[0], Fixture::Files(_)));

        let setup = match &fixtures[0] {
            Fixture::Files(setup) => setup,
            _ => unreachable!(),
        };

        assert_eq!(setup.files.len(), 1);
        assert_eq!(setup.files[0].src, fixture_dir2.join("source.conf"));
        assert_eq!(setup.files[0].dest, PathBuf::from("/etc/dest.conf"));
        assert!(!setup.files[0].raw);

        assert!(setup.root);
        assert!(setup.secrets.is_empty());
    }

    #[test]
    fn test_apply_files_fixture() {
        let test_dir = tempfile::tempdir().expect("Failed to create temporary directory");

        let file_content = "Hello, World!";

        let source_file = test_dir.path().join("source.conf");
        let dest_file = test_dir.path().join("dest.conf");

        std::fs::write(&source_file, file_content).unwrap();

        assert!(!dest_file.exists());

        let fixture = Fixture::Files(fixture::FilesSetup {
            files: vec![fixture::File {
                src: source_file.clone(),
                dest: dest_file.clone(),
                raw: true,
            }],
            root: false,
            secrets: Default::default(),
        });

        apply_fixtures(vec![fixture], false);

        assert!(dest_file.exists());

        let dest_content = std::fs::read_to_string(&dest_file).unwrap();

        assert_eq!(dest_content, file_content);
    }
}
