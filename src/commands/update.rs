use std::process::Command;

use crate::git::Git;
use crate::Update;

pub fn run(params: Update) -> Result<(), Box<dyn std::error::Error>> {
    let mut git = Git::open()?;

    if params.deps {
        let cargo_update = Command::new("cargo").arg("update").status()?;

        if !cargo_update.success() {
            return Err("Command `cargo update` failed!".into());
        }

        git.commit_files("Update Cargo.lock", &["Cargo.lock"])?;
    } else {
        todo!();
    }

    Ok(())
}