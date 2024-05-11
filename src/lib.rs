use std::path::PathBuf;

use fixture::Fixture;
use resolve_path::PathResolveExt;

mod fixture;
pub mod git;
mod repo;
mod template;

pub fn get_repo_dir() -> PathBuf {
    let config_dir = dirs::config_dir().unwrap();
    config_dir.join("spaceconf")
}

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
