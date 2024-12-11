//! Synchronise this crate's version with the `cairo-lang-*` crates.

use anyhow::{ensure, Result};
use clap::Parser;
use semver::{Prerelease, Version};
use toml_edit::{value, DocumentMut};
use xshell::{cmd, Shell};

/// Synchronise this crate's version with the `cairo-lang-*` crates.
#[derive(Default, Parser)]
pub struct Args {
    /// Do not edit any files, just inform what would be done.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    /// Set a custom value for the `build` metadata.
    #[arg(long)]
    pub build: Option<String>,

    /// Clear the pre-release identifier from the version.
    #[arg(long, default_value_t = false)]
    pub no_pre_release: bool,
}

pub fn main(args: Args) -> Result<()> {
    let sh = Shell::new()?;

    let mut cargo_toml = sh.read_file("Cargo.toml")?.parse::<DocumentMut>()?;

    let (package, is_workspace) = if cargo_toml.contains_key("workspace") {
        (
            cargo_toml["workspace"]["package"].as_table_mut().unwrap(),
            true,
        )
    } else {
        (cargo_toml["package"].as_table_mut().unwrap(), false)
    };

    let mut version = expected_version()?;

    if let Some(build) = args.build {
        version.build = build.parse()?;
    }
    if args.no_pre_release {
        version.pre = Prerelease::EMPTY;
    }

    package["version"] = value(version.to_string());

    eprintln!(
        "[{table}]\n{package}",
        table = if is_workspace {
            "workspace.package"
        } else {
            "package"
        }
    );

    if !args.dry_run {
        sh.write_file("Cargo.toml", cargo_toml.to_string())?;

        cmd!(sh, "cargo fetch").run()?;
    }

    Ok(())
}

/// Gets the version of the `cairo-lang-compiler` crate from `Cargo.lock`, which is the expected
/// version for the crate this script is being run on.
pub fn expected_version() -> Result<Version> {
    // NOTE: We are deliberately not using cargo_metadata to reduce build times of xtasks.

    let sh = Shell::new()?;
    let cargo_lock = sh.read_file("Cargo.lock")?.parse::<DocumentMut>()?;
    let packages = cargo_lock["package"].as_array_of_tables().unwrap();
    let compiler = {
        let pkgs = packages
            .into_iter()
            .filter(|pkg| pkg["name"].as_str().unwrap() == "cairo-lang-compiler")
            .collect::<Vec<_>>();
        ensure!(
            pkgs.len() == 1,
            "expected exactly one cairo-lang-compiler package in Cargo.lock, found: {}",
            pkgs.len()
        );
        pkgs.into_iter().next().unwrap()
    };
    let compiler_version = compiler["version"].as_str().unwrap();
    Ok(compiler_version.parse()?)
}