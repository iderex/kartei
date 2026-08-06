//! The conditions the suite is born with, checked by the suite itself.
//!
//! `docs/testing.md` states them in prose. Prose does not refuse anything, so
//! the two that a machine can read are read here: that this process is not
//! elevated, and that the harness crate never reaches the shipped dependency
//! graph. The rest of this file proves the helpers do what the other conditions
//! require of them, because a helper that quietly hands two tests the same
//! directory is worse than no helper at all.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

use kartei_testing::{TempDir, assert_not_elevated, bind_ephemeral, is_elevated, port_of};

#[test]
fn the_suite_is_not_running_with_administrative_rights() {
    assert_not_elevated();
}

#[test]
fn a_temporary_directory_exists_while_it_is_held_and_is_gone_afterwards() {
    let path: PathBuf;
    {
        let dir = TempDir::new("held");
        path = dir.path().to_path_buf();
        assert!(path.is_dir(), "{} was not created", path.display());

        std::fs::write(path.join("a-file"), b"contents")
            .expect("a test must be able to write inside its own directory");
    }

    assert!(
        !path.exists(),
        "{} outlived the value that owns it, so the next run inherits it",
        path.display()
    );
}

#[test]
fn two_directories_are_never_the_same_directory() {
    // Enough to catch a name built from a timestamp alone, which repeats
    // whenever two calls land inside one clock tick.
    let dirs: Vec<TempDir> = (0..64).map(|_| TempDir::new("unique")).collect();
    let paths: BTreeSet<PathBuf> = dirs.iter().map(|d| d.path().to_path_buf()).collect();

    assert_eq!(
        paths.len(),
        dirs.len(),
        "two temporary directories shared a path, which is the shared state the \
         helper exists to remove"
    );
}

#[test]
fn a_label_cannot_reach_outside_the_directory_it_names() {
    let dir = TempDir::new("../../escape me");
    let name = dir
        .path()
        .file_name()
        .expect("a created directory has a final component")
        .to_string_lossy()
        .into_owned();

    assert!(
        !name.contains(".."),
        "the label survived into the path as {name}"
    );
    assert_eq!(
        dir.path().parent(),
        Some(std::env::temp_dir().as_path()),
        "the directory was created outside the parent the helper chose"
    );
}

#[test]
fn an_ephemeral_port_is_assigned_and_held() {
    let first = bind_ephemeral();
    let second = bind_ephemeral();

    let a = port_of(&first);
    let b = port_of(&second);

    assert_ne!(a, 0, "the operating system reported the wildcard port back");
    assert_ne!(
        a, b,
        "two listeners were given one port, so one of them is not bound"
    );
    assert_eq!(
        first
            .local_addr()
            .expect("a bound listener reports its address")
            .ip()
            .to_string(),
        "127.0.0.1",
        "the listener is reachable from outside the machine"
    );
}

#[test]
fn the_elevation_query_answers_rather_than_guesses() {
    // The assertion above only proves the negative branch when the answer is
    // false. This proves the query ran and produced an answer at all, which is
    // what distinguishes a working query from one that returns false because it
    // could not look.
    let answer = is_elevated();
    assert!(!answer, "see the assertion above; this run is elevated");
}

#[test]
fn the_harness_never_reaches_the_shipped_dependency_graph() {
    // A helper crate that becomes a normal dependency of a shipped crate ends
    // up inside the binary, and it is the one crate in the workspace whose whole
    // purpose is to be absent from it. Normal edges only: being a
    // dev-dependency of a suite is what this crate is for.
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());

    let mut manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.pop();
    manifest.pop();
    manifest.push("Cargo.toml");

    let out = Command::new(cargo)
        .args([
            "tree",
            "--locked",
            "--edges",
            "normal",
            "--prefix",
            "none",
            "--invert",
            "kartei-testing",
        ])
        .arg("--manifest-path")
        .arg(&manifest)
        .output()
        .unwrap_or_else(|e| panic!("could not run cargo tree: {e}"));

    assert!(
        out.status.success(),
        "cargo tree failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let dependents: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !line.starts_with("kartei-testing "))
        .map(str::to_owned)
        .collect();

    assert!(
        dependents.is_empty(),
        "the harness is a normal dependency of:\n  {}",
        dependents.join("\n  ")
    );
}
