use std::{
    collections::HashMap,
    io::Write,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};

use log::error;
use termcolor::{ColorChoice, ColorSpec, StandardStream, WriteColor};

use crate::{
    fixture::{File, FilesSetup, Fixture, FixtureType},
    repo, template,
};

pub fn apply_fixtures(
    fixtures: Vec<Fixture>,
    revert: bool,
    no_backup: bool,
) -> std::io::Result<()> {
    let backup_dir = dirs::state_dir().unwrap().join("spaceconf");
    for fixture in fixtures {
        if fixture.skip() {
            continue;
        }

        match fixture.fixture_type {
            FixtureType::Files(setup) => {
                for file in setup.clone().files {
                    apply_file(
                        &file,
                        &backup_dir,
                        setup.root,
                        &setup.secrets,
                        revert,
                        no_backup,
                    )?;
                }
            }
            FixtureType::Repository(setup) => {
                repo::apply(setup.clone());
            }
        }
    }

    Ok(())
}

fn apply_file(
    file: &File,
    backup_dir: &Path,
    root: bool,
    secrets: &HashMap<String, String>,
    revert: bool,
    no_backup: bool,
) -> std::io::Result<()> {
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
    let Some(src) = file.src.clone().resolve() else {
        return Ok(());
    };
    let Some(dest) = file.dest.clone().resolve() else {
        return Ok(());
    };

    stdout.set_color(ColorSpec::new().set_fg(Some(termcolor::Color::White)))?;
    if revert {
        restore_file(backup_dir, &dest, root).inspect_err(|e| {
            eprintln!("Failed to restore {:?}: {}", dest, e);
        })
    } else {
        let output = if file.raw {
            std::fs::read_to_string(&src)
                .inspect_err(|_| error!("failed to read source file: {}", &src.to_string_lossy()))?
        } else {
            let input = std::fs::read_to_string(&src).inspect_err(|_| {
                error!("failed to read source file: {}", &src.to_string_lossy())
            })?;
            template::render(&input, secrets).unwrap()
        };

        if check_content(&output, &dest) {
            stdout.set_color(ColorSpec::new().set_fg(Some(termcolor::Color::Green)))?;
            writeln!(stdout, "{} is up to date", dest.to_string_lossy()).unwrap();
            stdout.set_color(ColorSpec::new().set_fg(Some(termcolor::Color::White)))?;
            return Ok(());
        }

        if !no_backup {
            if !backup_dir.exists() {
                std::fs::create_dir_all(backup_dir).inspect_err(|_| {
                    error!(
                        "failed to create parent directory(s): {}",
                        &backup_dir.to_string_lossy()
                    )
                })?;
            }
            backup_file(backup_dir, &dest);
        }

        let mode = src.metadata().unwrap().mode();
        if root {
            write_root(&dest, &output, mode)?;
        } else {
            let mut open_options = std::fs::OpenOptions::new();
            open_options.mode(mode);

            std::fs::create_dir_all(dest.parent().unwrap()).inspect_err(|_| {
                error!(
                    "failed to create parent directory(s): {}",
                    &dest.to_string_lossy()
                )
            })?;
            let mut file = open_options
                .write(true)
                .truncate(true)
                .create(true)
                .open(&dest)
                .inspect_err(|_| {
                    error!(
                        "failed to open destination file: {}",
                        &dest.to_string_lossy()
                    )
                })?;
            file.write_all(output.as_bytes()).inspect_err(|_| {
                error!(
                    "failed to write to destination file: {}",
                    &dest.to_string_lossy()
                )
            })?;
        }
        println!("Applying {:?}", dest);
        Ok(())
    }
}

fn check_content(content: &str, output: &PathBuf) -> bool {
    if !output.exists() {
        return false;
    }

    let existing_content = std::fs::read_to_string(output).unwrap();

    content == existing_content
}

fn backup_file(backup_dir: &Path, file: &PathBuf) {
    if !file.exists() {
        return;
    }

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

    let mode = backup_file.metadata().unwrap().mode();

    if root {
        write_root(file, &std::fs::read_to_string(backup_file).unwrap(), mode)?;
    } else {
        std::fs::copy(backup_file, file).unwrap();
    }
    Ok(())
}

fn get_backup_filename(backup_dir: &Path, file: &Path) -> PathBuf {
    backup_dir.join(file.strip_prefix("/").unwrap())
}

fn write_root(file: &PathBuf, content: &str, mode: u32) -> std::io::Result<()> {
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

    std::process::Command::new("sudo")
        .arg("chmod")
        .arg(format!("{:o}", mode))
        .arg(file)
        .status()
        .unwrap();

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::fixture::{self, FileDefinition, Fixture};

    #[test]
    fn test_apply_files_fixture() {
        let test_dir = tempfile::tempdir().expect("Failed to create temporary directory");

        let file_content = "Hello, World!";

        let source_file = test_dir.path().join("source.conf");
        let dest_file = test_dir.path().join("dest.conf");

        std::fs::write(&source_file, file_content).unwrap();

        assert!(!dest_file.exists());

        let fixture = Fixture {
            name: "test-fixture".into(),
            include_for: None,
            exclude_for: None,
            fixture_type: FixtureType::Files(fixture::FilesSetup {
                files: vec![fixture::File {
                    src: FileDefinition::Single(source_file.clone()),
                    dest: FileDefinition::Single(dest_file.clone()),
                    raw: true,
                    optional: false,
                }],
                root: false,
                secrets: Default::default(),
            }),
        };

        apply_fixtures(vec![fixture], false, true).unwrap();

        assert!(dest_file.exists());

        let dest_content = std::fs::read_to_string(&dest_file).unwrap();

        assert_eq!(dest_content, file_content);
    }

    #[test]
    fn test_apply_excluded_fixture() {
        let test_dir = tempfile::tempdir().expect("Failed to create temporary directory");

        let file_content = "Hello, World!";

        let source_file = test_dir.path().join("source.conf");
        let dest_file = test_dir.path().join("dest.conf");

        std::fs::write(&source_file, file_content).unwrap();

        assert!(!dest_file.exists());

        let fixture = Fixture {
            name: "test-fixture".into(),
            include_for: None,
            exclude_for: Some(vec![std::env::consts::OS.into()]),
            fixture_type: FixtureType::Files(fixture::FilesSetup {
                files: vec![fixture::File {
                    src: FileDefinition::Single(source_file.clone()),
                    dest: FileDefinition::Single(dest_file.clone()),
                    raw: true,
                    optional: false,
                }],
                root: false,
                secrets: Default::default(),
            }),
        };

        apply_fixtures(vec![fixture], false, true).unwrap();

        assert!(!dest_file.exists());
    }

    #[test]
    fn test_apply_unresolved_source() {
        let test_dir = tempfile::tempdir().expect("Failed to create temporary directory");

        let file_content = "Hello, World!";

        let source_file = test_dir.path().join("source.conf");
        let dest_file = test_dir.path().join("dest.conf");

        std::fs::write(&source_file, file_content).unwrap();

        assert!(!dest_file.exists());

        let fixture = Fixture {
            name: "test-fixture".into(),
            include_for: None,
            exclude_for: None,
            fixture_type: FixtureType::Files(fixture::FilesSetup {
                files: vec![fixture::File {
                    src: FileDefinition::Multiple(HashMap::from_iter(vec![(
                        "nonexistent".into(),
                        source_file.clone(),
                    )])),
                    dest: FileDefinition::Single(dest_file.clone()),
                    raw: true,
                    optional: true,
                }],
                root: false,
                secrets: Default::default(),
            }),
        };

        apply_fixtures(vec![fixture], false, true).unwrap();

        assert!(!dest_file.exists());
    }

    #[test]
    fn test_apply_unresolved_destination() {
        let test_dir = tempfile::tempdir().expect("Failed to create temporary directory");

        let file_content = "Hello, World!";

        let source_file = test_dir.path().join("source.conf");
        let dest_file = test_dir.path().join("dest.conf");

        std::fs::write(&source_file, file_content).unwrap();

        assert!(!dest_file.exists());

        let fixture = Fixture {
            name: "test-fixture".into(),
            include_for: None,
            exclude_for: None,
            fixture_type: FixtureType::Files(fixture::FilesSetup {
                files: vec![fixture::File {
                    src: FileDefinition::Single(source_file.clone()),
                    dest: FileDefinition::Multiple(HashMap::from_iter(vec![(
                        "nonexistent".into(),
                        dest_file.clone(),
                    )])),
                    raw: true,
                    optional: true,
                }],
                root: false,
                secrets: Default::default(),
            }),
        };

        apply_fixtures(vec![fixture], false, true).unwrap();

        assert!(!dest_file.exists());
    }

    #[test]
    fn test_apply_missing_parent_dirs() {
        let test_dir = tempfile::tempdir().expect("Failed to create temporary directory");

        let src_path = test_dir.path().join("source.conf");
        let dest_path = test_dir.path().join("nested/dest.conf");

        let file = File {
            src: FileDefinition::Single(src_path.clone()),
            dest: FileDefinition::Single(dest_path.clone()),
            raw: false,
            optional: false,
        };

        std::fs::write(&src_path, "Hello, World!").unwrap();

        assert!(!dest_path.parent().unwrap().exists());
        assert!(!dest_path.exists());

        apply_file(&file, test_dir.path(), false, &HashMap::new(), false, true).unwrap();

        assert!(dest_path.parent().unwrap().exists());
        assert!(dest_path.exists());
    }

    #[test]
    fn test_apply_preserves_mode() {
        let test_dir = tempfile::tempdir().expect("Failed to create temporary directory");

        let src_path = test_dir.path().join("source.conf");
        let dest_path = test_dir.path().join("dest.conf");

        let file = File {
            src: FileDefinition::Single(src_path.clone()),
            dest: FileDefinition::Single(dest_path.clone()),
            raw: false,
            optional: false,
        };

        let mode = 0o600;

        let mut open_options = std::fs::OpenOptions::new();
        open_options.mode(mode);
        open_options.write(true);
        open_options.create(true);
        let mut src_file = open_options.open(&src_path).unwrap();
        src_file.write_all(b"Hello, World!").unwrap();

        apply_file(&file, test_dir.path(), false, &HashMap::new(), false, true).unwrap();

        let dest_metadata = std::fs::metadata(&dest_path).unwrap();
        assert_eq!(dest_metadata.mode() & 0o777, mode);
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
