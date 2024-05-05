use std::{path::PathBuf, process::Command};

use crate::fixture::Reference;

pub fn clone(repo: &str, path: &PathBuf, reference: Option<Reference>) {
    let mut cmd = Command::new("git");
    cmd.arg("clone");
    cmd.arg(repo);
    cmd.arg(path);

    match reference {
        Some(Reference::Branch(branch)) => {
            cmd.arg("--branch").arg(branch);
        }
        Some(Reference::Tag(tag)) => {
            cmd.arg("--branch").arg(tag);
        }
        Some(Reference::Commit(commit)) => {
            cmd.arg("--branch").arg(commit);
            // TODO: idk if this is correct
        }
        None => {}
    }

    cmd.output().expect("failed to execute git clone");
}

pub fn pull(path: &PathBuf) {
    let mut cmd = Command::new("git");
    cmd.arg("pull");
    cmd.current_dir(path);
    cmd.output().expect("failed to execute git pull");
}
