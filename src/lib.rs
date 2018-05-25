extern crate cargo;
extern crate quicli;
extern crate time;
#[macro_use]
extern crate failure;

pub mod cargo_utils;
pub mod ebuild;

use quicli::prelude::*;

#[derive(Debug, StructOpt)]
pub enum Command {
    #[structopt(name = "build")]
    /// Build an ebuild file from a cargo project
    Build {
        #[structopt(long = "unstable-flags", short = "Z")]
        unstable_flags: Vec<String>,
    },
}

use cargo::Config;
use cargo_utils::*;
use ebuild::*;
use failure::err_msg;
use failure::Error;
use std::result;

/// Parse cli commands
pub fn run_cargo_ebuild(
    cmd: Option<Command>,
    verbosity: u8,
    quiet: bool,
) -> result::Result<Ebuild, Error> {
    // If no command is specified run build with default conf
    let cmd = cmd.unwrap_or(Command::Build {
        unstable_flags: Vec::new(),
    });

    // Here will be the match of the commands, now just example
    match cmd {
        Command::Build { unstable_flags } => build(u32::from(verbosity), quiet, unstable_flags),
    }
}

/// Quite huge all-in-one func that generates and Ebuild from some cli
/// cli configuration
fn build(verbosity: u32, quiet: bool, flags: Vec<String>) -> result::Result<Ebuild, Error> {
    // build the crate URIs
    let mut config = Config::default()?;

    // setup cargo configuration
    config.configure(
        u32::from(verbosity),
        Some(quiet),
        &None, // color
        false, // frozen
        false, // locked
        flags.as_ref(),
    )?;

    // Build up data about the package we are attempting to generate a recipe for
    let md = PackageInfo::new(&config)?;

    // Our current package
    let package = md.package()?;

    // Look for Cargo.toml parent
    let _crate_root = package
        .manifest_path()
        .parent()
        .ok_or_else(|| err_msg(format_err!("Cargo.toml must have a parent")))?;

    // Resolve all dependencies (generate or use Cargo.lock as necessary)
    let resolve = md.resolve()?;

    let mut git_crates = Vec::new();

    // build the crates the package needs
    let mut crates = resolve
        .1
        .iter()
        .filter_map(|pkg| {
            // get the source info for this package
            let src_id = pkg.source_id();
            if src_id.is_registry() {
                // this package appears in a crate registry
                Some(format!("{}-{}\n", pkg.name(), pkg.version()))
            } else if src_id.is_path() {
                // we don't want to spit out path based
                // entries since they're within the crate
                // we are packaging
                None
            } else if src_id.is_git() {
                use cargo::sources::GitSource;

                match GitSource::new(&src_id, &config) {
                    Ok(git_src) => {
                        git_crates.push(format!(
                            "{} {}\n",
                            git_src.url(),
                            // not sure on how is the best way to encode this
                            // a git_reference could be a tag, branch, or revision
                            src_id.git_reference().unwrap().pretty_ref().unwrap()
                        ))
                    }
                    Err(err) => error!(
                        "Not able to find git source for {} caused by {}",
                        pkg.name(),
                        err
                    ),
                };

                None
            } else if src_id.is_alt_registry() {
                Some(format!("{} \\\n", src_id.url().to_string()))
            } else {
                warn!("There is no method to fetch package {}", pkg.name());
                None
            }
        })
        .collect::<Vec<String>>();

    // sort the crates
    crates.sort();

    if !git_crates.is_empty() {
        info!("Found this git crates: {:?}", git_crates);
    }

    // root package metadata
    let metadata = package.manifest().metadata();

    // package description is used as BitBake summary
    let summary = metadata.description.as_ref().map_or_else(
        || {
            debug!("No package.description set in your Cargo.toml, using package.name");
            package.name().to_string()
        },
        |s| s.to_string(),
    );

    // package homepage (or source code location)
    let homepage = metadata.homepage.as_ref().or_else(|| {
        debug!("No package.homepage set in your Cargo.toml, trying package.repository");
        metadata.repository.as_ref().or_else(|| {
            debug!("No package.repository set in your Cargo.toml");
            None
        })
    });

    // package version
    let version = package.manifest().version().to_string();

    let license = metadata
        .license
        .as_ref()
        .cloned()
        .unwrap_or_else(|| String::from("unknown license"));

    // write the contents out
    Ok(Ebuild::new(
        &package.name().to_string(),
        &summary,
        homepage,
        license.trim(),
        crates.join("").as_ref(),
        git_crates.join("").as_ref(),
        version.as_ref(),
        "cargo-ebuild",
        env!("CARGO_PKG_VERSION"),
        1900 + time::now().tm_year,
    ))
}
