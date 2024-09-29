use std::path::PathBuf;

use clap::{Parser, Subcommand};

use spaceconf::git;
use spaceconf::list_fixtures;
use spaceconf::ops::apply::apply_fixtures;
use spaceconf::ops::diff::diff_fixtures;
use spaceconf::ops::load::load_fixtures;

#[derive(Parser)]
#[command(version, about)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Clone repository
    Clone(CloneArgs),

    /// Apply configuration to the system
    Apply(ApplyArgs),

    /// List all available fixtures
    List,

    /// Show differences between the current configuration and the fixtures
    Diff,
}

#[derive(Parser)]
struct CloneArgs {
    /// Repository URL
    repository: String,
}

#[derive(Parser)]
struct ApplyArgs {
    /// List of fixtures to apply
    fixtures: Vec<String>,

    /// Revert the specified configuration
    #[arg(short, long)]
    revert: bool,

    /// Do not create a backup of the current configuration
    #[arg(short, long)]
    no_backup: bool,
}

fn main() {
    env_logger::init();

    let cli = Args::parse();

    let repo_dir = get_repo_dir();

    if let Command::Clone(args) = &cli.command {
        if repo_dir.exists() {
            eprintln!("Repository already exists");
            std::process::exit(1);
        }

        println!("Cloning repository to ~/.spaceconf ...");
        git::clone(&args.repository, &repo_dir, None);
        std::process::exit(0);
    }

    if !repo_dir.exists() {
        eprintln!("Repository does not exist, please run 'spaceconf clone <repo URL>' first");
        std::process::exit(1);
    }

    let fixture_names = match cli.command {
        Command::Apply(ref args) => args.fixtures.clone(),
        _ => vec![],
    };

    let fixtures = load_fixtures(get_repo_dir(), fixture_names).unwrap();

    match cli.command {
        Command::List => {
            list_fixtures(fixtures);
        }
        Command::Apply(args) => match apply_fixtures(fixtures, args.revert, args.no_backup) {
            Ok(_) => println!("Configuration applied successfully"),
            Err(e) => eprintln!("Error: {}", e),
        },
        Command::Diff => diff_fixtures(fixtures),
        _ => unimplemented!(),
    }
}

fn get_repo_dir() -> PathBuf {
    dirs::home_dir().unwrap().join(".spaceconf")
}
