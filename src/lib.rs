use std::path::PathBuf;

use fixture::Fixture;
use resolve_path::PathResolveExt;

mod fixture;
pub mod git;
mod repo;
mod template;

pub fn load_fixtures(dir: PathBuf) -> std::io::Result<Vec<Fixture>> {
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
                    } else {
                        let output = if file.raw {
                            std::fs::read_to_string(&file.src).unwrap()
                        } else {
                            let input = std::fs::read_to_string(&file.src).unwrap();
                            template::render(&input, &setup.secrets).unwrap()
                        };

                        println!("Applying {:?}", file.dest);
                        if setup.root {
                            #[cfg(not(target_os = "linux"))]
                            {
                                eprintln!("Root fixture is currently only supported on Linux");
                                std::process::exit(1);
                            }

                            let temp_file = PathBuf::from(format!(
                                "/tmp/spaceconf-{}.tmp",
                                uuid::Uuid::new_v4()
                            ));
                            std::fs::write(&temp_file, output).unwrap();
                            std::process::Command::new("sudo")
                                .arg("cp")
                                .arg(&temp_file)
                                .arg(&file.dest)
                                .status()
                                .unwrap();
                        } else {
                            std::fs::write(&file.dest, output).unwrap();
                        }
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

        let fixtures = load_fixtures(test_dir.path().to_path_buf()).unwrap();

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
