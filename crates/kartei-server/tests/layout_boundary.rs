//! The layout is only a boundary if something refuses a crossing.
//!
//! The workspace layout says which crate is allowed to know about which. That
//! is a sentence in a doc comment, and a sentence does not refuse anything: one
//! line added to one `[dependencies]` table turns the model into something that
//! knows about sync, and nothing announces it. This test reads the real
//! dependency graph with `cargo tree` and refuses the crossings the layout
//! forbids.
//!
//! It lives in the binary crate because that is the one crate the layout allows
//! to name every other one, so a test that has to name them all does not itself
//! cross a boundary.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

/// What each crate's normal dependency graph may not contain.
///
/// Every entry is a sentence from the layout, not a preference. The model
/// knows nothing about storage or sync, so neither may appear beneath it. An
/// importer depends on the model only, so everything else in the workspace is
/// forbidden beneath it: an importer that could reach storage would be able to
/// write a document that never passed through the apply path.
const FORBIDDEN: &[(&str, &[&str])] = &[
    (
        "kartei-model",
        &[
            "kartei-sync",
            "kartei-store",
            "kartei-proto",
            "kartei-import",
            "kartei-server",
        ],
    ),
    (
        "kartei-import",
        &[
            "kartei-sync",
            "kartei-store",
            "kartei-proto",
            "kartei-server",
        ],
    ),
    ("kartei-store", &["kartei-import", "kartei-server"]),
    ("kartei-proto", &["kartei-store", "kartei-server"]),
];

/// The crates that must never reach a replication engine, whatever engine is
/// chosen.
const ENGINE_FREE: &[&str] = &["kartei-model", "kartei-store", "kartei-proto"];

/// The engine crates, by the name they carry on the dependency graph.
///
/// Empty, because the engine is not chosen yet and no crate in the workspace
/// depends on one. The leg that reads this set therefore asserts nothing today
/// and says so when it runs, so a green run is not readable as a checked one.
/// When the engine decision is recorded, its crate name goes here and the leg
/// starts biting.
const ENGINE_CRATES: &[&str] = &[];

fn workspace_root() -> PathBuf {
    // crates/kartei-server -> crates -> the workspace root.
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.pop();
    root.pop();
    root
}

/// Every crate in `package`'s normal dependency graph, including `package`.
///
/// Normal edges only: a dev-dependency is a dependency of the tests and not of
/// the artefact, and forbidding one would refuse a test that legitimately
/// builds a document to project.
fn normal_dependencies(package: &str) -> BTreeSet<String> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let mut manifest = workspace_root();
    manifest.push("Cargo.toml");

    let out = Command::new(cargo)
        .args(["tree", "--locked", "--edges", "normal", "--prefix", "none"])
        .arg("--manifest-path")
        .arg(&manifest)
        .args(["--package", package])
        .output()
        .unwrap_or_else(|e| panic!("could not run cargo tree for {package}: {e}"));

    assert!(
        out.status.success(),
        "cargo tree failed for {package}:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .map(str::to_owned)
        .collect()
}

#[test]
fn no_crate_depends_on_what_the_layout_forbids_it() {
    let mut crossings = Vec::new();

    for (package, forbidden) in FORBIDDEN {
        let deps = normal_dependencies(package);
        for name in *forbidden {
            if deps.contains(*name) {
                crossings.push(format!("{package} depends on {name}"));
            }
        }
    }

    assert!(
        crossings.is_empty(),
        "the dependency graph crosses a boundary the layout forbids:\n  {}",
        crossings.join("\n  ")
    );
}

#[test]
fn no_engine_reaches_a_crate_above_the_boundary() {
    if ENGINE_CRATES.is_empty() {
        println!(
            "engine set is empty: no engine is chosen and none is depended on, \
             so this leg asserted nothing about {} crate(s)",
            ENGINE_FREE.len()
        );
        return;
    }

    let mut crossings = Vec::new();

    for package in ENGINE_FREE {
        let deps = normal_dependencies(package);
        for engine in ENGINE_CRATES {
            if deps.contains(*engine) {
                crossings.push(format!("{package} reaches the engine crate {engine}"));
            }
        }
    }

    assert!(
        crossings.is_empty(),
        "an engine reached a crate that is not allowed to name one:\n  {}",
        crossings.join("\n  ")
    );
}
