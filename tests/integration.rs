extern crate assert_cli;
extern crate tempdir;

use std::fs::File;
use std::io::prelude::*;
use tempdir::TempDir;

#[test]
fn build() {
    let ebuild_name = format!("cargo-ebuild-{}.ebuild", env!("CARGO_PKG_VERSION"));

    let build_dir = TempDir::new("build").unwrap();
    let build_path = build_dir.path().join(&ebuild_name);

    assert_cli::Assert::main_binary()
        .with_args(&[build_dir.path().to_str().unwrap(), "build"])
        .stdout()
        .is((format!("Wrote: {}", build_path.display())).as_str())
        .unwrap();

    let mut build_file = match File::open(build_path) {
        Err(why) => panic!("couldn't open generated ebuild: {}", why),
        Ok(f) => f,
    };

    let mut new_ebuild = String::new();
    build_file.read_to_string(&mut new_ebuild).unwrap();

    let mut test_file = match File::open(format!("tests/{}", ebuild_name)) {
        Err(why) => panic!("couldn't open test ebuild: {}", why),
        Ok(f) => f,
    };

    let mut test_ebuild = String::new();
    test_file.read_to_string(&mut test_ebuild).unwrap();

    assert_eq!(new_ebuild, test_ebuild);
}
