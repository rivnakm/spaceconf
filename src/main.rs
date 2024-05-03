use std::path::PathBuf;

use clap::{Parser, Subcommand};
use fixture::Fixture;
use resolve_path::PathResolveExt;

mod fixture;

#[derive(Parser)]
#[command(version, about)]
struct Args {
    #[command(subcommand)]
    command: Command,

    /// Operate on system configuration files
    #[arg(short, long)]
    system: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Synchronize repository with remote
    Sync,

    /// Save local configuration to repository
    Save,

    /// Apply configuration to the system
    Apply(ApplyArgs),

    /// List all available fixtures
    List,

    /// Check if local configuration is up-to-date
    Check,
}

#[derive(Parser)]
struct ApplyArgs {
    /// Apply only the specified fixture
    #[arg(short, long)]
    fixture: Option<String>,

    /// Revert the specified configuration
    #[arg(short, long)]
    revert: bool,
}

fn main() {
    let cli = Args::parse();

    // TODO: check if there's a new commit in the remote repo, warn the user, and exit

    if cli.system {
        sudo::escalate_if_needed().expect("Failed to acquire root privileges");
    }

    let fixtures = load_fixtures(cli.system).unwrap();

    match cli.command {
        Command::List => {
            list_fixtures(fixtures);
        }
        Command::Check => {
            check_fixtures(fixtures);
        }
        Command::Apply(args) => {
            apply_fixtures(fixtures, args);
        }
        _ => unimplemented!(),
    }
}

fn get_repo_dir() -> PathBuf {
    let config_dir = dirs::config_dir().unwrap();
    config_dir.join("spaceconf")
}

fn get_fixtures() -> std::io::Result<Vec<Fixture>> {
    let repo_dir = get_repo_dir();
    let dir_entries = std::fs::read_dir(repo_dir)?;
    let fixture_dirs: Vec<PathBuf> = dir_entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
        .filter(|entry| {
            let fixure_dir_entries = std::fs::read_dir(entry.path()).unwrap();
            fixure_dir_entries
                .filter_map(|entry| entry.ok())
                .any(|entry| entry.file_name() == "fixture.toml")
        })
        .map(|entry| entry.path())
        .collect();

    let fixtures = fixture_dirs
        .into_iter()
        .map(|fixture_dir| {
            let fixture_file = fixture_dir.join("fixture.toml");
            let fixture = std::fs::read_to_string(fixture_file).unwrap();
            let mut fixture: Fixture = toml::from_str(&fixture).unwrap();
            match &mut fixture.r#type {
                fixture::FixtureType::Files(ref mut setup) => {
                    for file in &mut setup.files {
                        file.src = fixture_dir.join(&file.src);
                        file.dest = file.dest.resolve().to_path_buf();
                    }
                }
                fixture::FixtureType::Repository(_) => {
                    unimplemented!();
                }
            }
            fixture
        })
        .collect();

    Ok(fixtures)
}

fn load_fixtures(root: bool) -> std::io::Result<Vec<Fixture>> {
    let fixtures = get_fixtures()?;

    Ok(fixtures
        .into_iter()
        .filter(|fixture| match &fixture.r#type {
            fixture::FixtureType::Files(setup) => setup.root == root,
            fixture::FixtureType::Repository(_) => !root,
        })
        .collect())
}

fn list_fixtures(fixtures: Vec<Fixture>) {
    for fixture in fixtures {
        println!("{:?}", fixture);
    }
}

fn check_fixtures(fixtures: Vec<Fixture>) {
    for fixture in fixtures {
        match &fixture.r#type {
            fixture::FixtureType::Files(setup) => {
                for file in &setup.files {
                    if !file.dest.exists() {
                        println!("{:?} does not exist", file.dest);
                        continue;
                    }

                    let src_hash = crc32fast::hash(std::fs::read(&file.src).unwrap().as_slice());
                    let dest_hash = crc32fast::hash(std::fs::read(&file.dest).unwrap().as_slice());

                    if src_hash == dest_hash {
                        println!("{:?} is up to date", file.dest);
                    } else {
                        println!("{:?} is out of date", file.dest);
                    }
                }
            }
            fixture::FixtureType::Repository(_) => {
                unimplemented!("'check' command is not supported for repository fixtures");
            }
        }
    }
}

fn apply_fixtures(fixtures: Vec<Fixture>, args: ApplyArgs) {
    for fixture in fixtures {
        match &fixture.r#type {
            fixture::FixtureType::Files(setup) => {
                for file in &setup.files {
                    if args.revert {
                        unimplemented!("store the original file content and restore it here");
                    } else {
                        std::fs::copy(&file.src, &file.dest).unwrap();
                    }
                }
            }
            fixture::FixtureType::Repository(_) => {
                unimplemented!("'apply' command is not supported for repository fixtures");
            }
        }
    }
}
