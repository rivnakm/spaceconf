use std::path::{Path, PathBuf};

use fixture::Fixture;
use log::error;
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
                if setup.root {
                    println!("  Root: true");
                }
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

pub fn apply_fixtures(
    fixtures: Vec<Fixture>,
    revert: bool,
    no_backup: bool,
) -> std::io::Result<()> {
    let backup_dir = dirs::state_dir().unwrap().join("spaceconf");
    for fixture in fixtures {
        match &fixture {
            fixture::Fixture::Files(setup) => {
                for file in &setup.files {
                    if revert {
                        if let Err(e) = restore_file(&backup_dir, &file.dest, setup.root) {
                            eprintln!("Failed to restore {:?}: {}", file.dest, e);
                            return Err(e);
                        }
                    } else {
                        let output = if file.raw {
                            std::fs::read_to_string(&file.src).inspect_err(|_| {
                                error!(
                                    "failed to read source file: {}",
                                    &file.src.to_string_lossy()
                                )
                            })?
                        } else {
                            let input = std::fs::read_to_string(&file.src).inspect_err(|_| {
                                error!(
                                    "failed to read source file: {}",
                                    &file.src.to_string_lossy()
                                )
                            })?;
                            template::render(&input, &setup.secrets).unwrap()
                        };

                        if !no_backup {
                            if !backup_dir.exists() {
                                std::fs::create_dir_all(&backup_dir).inspect_err(|_| {
                                    error!(
                                        "failed to create parent directorie(s): {}",
                                        &backup_dir.to_string_lossy()
                                    )
                                })?;
                            }
                            backup_file(&backup_dir, &file.dest);
                        }

                        if setup.root {
                            write_root(&file.dest, &output)?;
                        } else {
                            std::fs::write(&file.dest, output).inspect_err(|_| {
                                error!(
                                    "failed to read destination file: {}",
                                    &file.dest.to_string_lossy()
                                )
                            })?;
                        }
                        println!("Applying {:?}", file.dest);
                    }
                }
            }
            fixture::Fixture::Repository(setup) => {
                repo::apply(setup.clone());
            }
        }
    }

    Ok(())
}

fn backup_file(backup_dir: &Path, file: &PathBuf) {
    let backup_file = get_backup_filename(backup_dir, file);
    std::fs::create_dir_all(backup_file.parent().unwrap()).unwrap();
    std::fs::copy(file, backup_file).unwrap();
}

fn restore_file(backup_dir: &Path, file: &PathBuf, root: bool) -> std::io::Result<()> {
    let backup_file = get_backup_filename(backup_dir, file);
    if !backup_file.exists() {
        eprintln!("Backup file does not exist for {:?}", file);
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Backup file does not exist",
        ));
    }

    if root {
        write_root(file, &std::fs::read_to_string(backup_file).unwrap())?;
    } else {
        std::fs::copy(backup_file, file).unwrap();
    }
    Ok(())
}

fn get_backup_filename(backup_dir: &Path, file: &Path) -> PathBuf {
    backup_dir.join(file.strip_prefix("/").unwrap())
}

fn write_root(file: &PathBuf, content: &str) -> std::io::Result<()> {
    #[cfg(not(target_os = "linux"))]
    {
        eprintln!("Root fixture is currently only supported on Linux");
        std::process::exit(1);
    }

    let temp_file = PathBuf::from(format!("/tmp/spaceconf-{}.tmp", uuid::Uuid::new_v4()));
    std::fs::write(&temp_file, content).inspect_err(|_| {
        error!(
            "failed to write temporary file: {}",
            &temp_file.to_string_lossy()
        )
    })?;
    if !file.parent().unwrap().exists() {
        std::process::Command::new("sudo")
            .arg("mkdir")
            .arg("-p")
            .arg(file.parent().unwrap())
            .status()
            .unwrap();
    }
    std::process::Command::new("sudo")
        .arg("cp")
        .arg(&temp_file)
        .arg(file)
        .status()
        .unwrap();
    Ok(())
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

        apply_fixtures(vec![fixture], false, true).unwrap();

        assert!(dest_file.exists());

        let dest_content = std::fs::read_to_string(&dest_file).unwrap();

        assert_eq!(dest_content, file_content);
    }

    #[test]
    fn test_backup_file() {
        let test_dir = tempfile::tempdir().expect("Failed to create temporary directory");

        let backup_dir = test_dir.path().join("backup");
        std::fs::create_dir(&backup_dir).unwrap();

        let file = test_dir.path().join("file.txt");
        std::fs::write(&file, "Hello, World!").unwrap();

        let backup_filename = backup_dir.join(file.strip_prefix("/").unwrap());

        assert!(!backup_filename.exists());

        backup_file(&backup_dir, &file);

        assert!(backup_filename.exists());
    }

    #[test]
    fn test_restore_file() {
        let test_dir = tempfile::tempdir().expect("Failed to create temporary directory");

        let backup_dir = test_dir.path().join("backup");
        std::fs::create_dir(&backup_dir).unwrap();

        let file = test_dir.path().join("file.txt");
        std::fs::write(file.clone(), "Hello, World!").unwrap();

        let backup_filename = backup_dir.join(file.strip_prefix("/").unwrap());
        std::fs::create_dir_all(backup_filename.parent().unwrap()).unwrap();
        std::fs::write(backup_filename, "Hello, Backup!").unwrap();

        let restored_file = test_dir.path().join("file.txt");

        let pre_restore_content = std::fs::read_to_string(&restored_file).unwrap();

        assert_eq!(pre_restore_content, "Hello, World!");

        restore_file(&backup_dir, &restored_file, false).unwrap();

        assert!(restored_file.exists());

        let restored_content = std::fs::read_to_string(&restored_file).unwrap();

        assert_eq!(restored_content, "Hello, Backup!");
    }
}
