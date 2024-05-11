use clap::{Parser, Subcommand};

use spaceconf::git;
use spaceconf::{apply_fixtures, check_fixtures, get_repo_dir, list_fixtures, load_fixtures};

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
    /// Clone repository
    Clone(CloneArgs),

    /// Apply configuration to the system
    Apply(ApplyArgs),

    /// List all available fixtures
    List,

    /// Check if local configuration is up-to-date
    Check,
}

#[derive(Parser)]
struct CloneArgs {
    /// Repository URL
    repository: String,
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

    let repo_dir = get_repo_dir();

    if let Command::Clone(args) = &cli.command {
        if repo_dir.exists() {
            eprintln!("Repository already exists, please run 'spaceconf sync' instead");
            std::process::exit(1);
        }

        println!("Cloning repository...");
        git::clone(&args.repository, &repo_dir, None);
        std::process::exit(0);
    }

    if !repo_dir.exists() {
        eprintln!("Repository does not exist, please run 'spaceconf clone <repo URL>' first");
        std::process::exit(1);
    }

    if cli.system {
        sudo::escalate_if_needed().expect("Failed to acquire root privileges");
    }

    let fixtures = load_fixtures(get_repo_dir(), cli.system).unwrap();

    match cli.command {
        Command::List => {
            list_fixtures(fixtures);
        }
        Command::Check => {
            check_fixtures(fixtures);
        }
        Command::Apply(args) => {
            apply_fixtures(fixtures, args.revert);
        }
        _ => unimplemented!(),
    }
}
