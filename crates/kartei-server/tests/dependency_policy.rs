// kartei, a self-hosted workspace for documents and structured data.
// Copyright (C) 2026  iderex
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! The dependency policy in `deny.toml` is watched refusing, on the real file.
//!
//! An allowed set is a list until something is seen being turned away by it.
//! This suite writes fixture workspaces of its own, points the repository's own
//! policy at them, and asserts which ones are refused and for which reason.
//!
//! The policy under test is the real one. The fixtures are fixtures. That split
//! is deliberate: a leg judging this repository's own dependency graph would
//! prove the state of the tree on the day it ran, and the tree has no
//! third-party package at all, so such a leg would pass by having nothing to
//! look at.
//!
//! ## The near-miss
//!
//! `GPL-2.0-or-later` is allowed and `GPL-2.0-only` is refused, and that pair is
//! the whole of the licence demonstration because it is the mistake somebody
//! actually makes. The two spellings name the same licence text. The clause that
//! differs is the one permitting the code to be used under a later version,
//! which is exactly what lets it be combined into an AGPL-3.0 work, so the
//! difference between a dependency this product may carry and one it may not is
//! a suffix on a line in somebody else's manifest. A fixture holding an
//! obviously proprietary licence would red just as reliably and would prove far
//! less, because nobody adds one by accident.
//!
//! The second pair is the same shape for duplicates: one version of a crate
//! passes and two versions of it are refused, with nothing else changed.
//!
//! ## Where this runs
//!
//! Behind the `policy` feature, so `cargo test --workspace` does not build it.
//! It needs the `cargo-deny` binary, which the workspace does not otherwise
//! require, and a contributor who has not installed a tool should not be told
//! their suite is broken. The `dependency-policy` job installs it and runs this
//! target next to the real check, so the check demonstrates that it can fail
//! every time it reports that it did not.
//!
//! ## What this does not reach
//!
//! The advisories leg of the policy is not demonstrated here. Refusing a
//! vulnerable crate needs a crate with an advisory against it, which means the
//! advisory database and a real package from the registry, and both live on
//! somebody else's server. A test does not reach the network, so that half is
//! asserted by configuration and has not been watched biting.
//!
//! Nothing here judges whether the allowed set is the right one. That the set
//! refuses what it does not list is what this file shows. Whether the list
//! itself matches what AGPL-3.0 permits inbound is a legal judgement, it is
//! argued in the comments of `deny.toml`, and it is read by a person.

use std::path::{Path, PathBuf};
use std::process::Output;

use kartei_testing::TempDir;

/// A licence the policy allows, and one of the two halves of the near-miss.
const LICENCE_ALLOWED: &str = "GPL-2.0-or-later";
/// The same licence without the clause that makes it combinable, which the
/// policy therefore refuses.
const LICENCE_REFUSED: &str = "GPL-2.0-only";

fn workspace_root() -> PathBuf {
    // crates/kartei-server -> crates -> the workspace root.
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.pop();
    root.pop();
    root
}

/// Writes a fixture workspace and returns the path to its manifest.
///
/// The fixture is one package that depends on a second package by path, so the
/// whole graph is inside the directory this test created and resolving it
/// reaches nothing. `duplicated` adds a second copy of that package at a
/// different version, under a rename, which is how a real graph ends up
/// carrying two versions of one crate.
fn write_fixture(root: &Path, licence: &str, duplicated: bool) -> PathBuf {
    let dependencies = if duplicated {
        "old = { path = \"old\", package = \"borrowed\" }\n\
         new = { path = \"new\", package = \"borrowed\" }\n"
    } else {
        "borrowed = { path = \"old\" }\n"
    };

    let manifest = format!(
        "[package]\n\
         name = \"policy-fixture\"\n\
         version = \"0.1.0\"\n\
         edition = \"2021\"\n\
         license = \"AGPL-3.0-only\"\n\
         \n\
         [dependencies]\n\
         {dependencies}"
    );

    std::fs::create_dir_all(root.join("src")).expect("the fixture directory is the test's own");
    std::fs::write(root.join("Cargo.toml"), manifest).expect("writing the fixture manifest");
    std::fs::write(
        root.join("src").join("lib.rs"),
        "// nothing to compile but a crate\n",
    )
    .expect("writing the fixture source");

    write_dependency(root, "old", "0.1.0", licence);
    if duplicated {
        write_dependency(root, "new", "0.2.0", licence);
    }

    root.join("Cargo.toml")
}

/// Writes one of the depended-on packages: a manifest and an empty library.
///
/// The package name is the same in both, which is what makes two of them a
/// duplicate rather than two unrelated crates.
fn write_dependency(root: &Path, at: &str, version: &str, licence: &str) {
    let dir = root.join(at);

    std::fs::create_dir_all(dir.join("src")).expect("the fixture directory is the test's own");
    std::fs::write(
        dir.join("Cargo.toml"),
        format!(
            "[package]\n\
             name = \"borrowed\"\n\
             version = \"{version}\"\n\
             edition = \"2021\"\n\
             license = \"{licence}\"\n"
        ),
    )
    .expect("writing a fixture manifest");
    std::fs::write(
        dir.join("src").join("lib.rs"),
        "// nothing to compile but a crate\n",
    )
    .expect("writing a fixture source");
}

/// Runs the repository's own policy against a fixture.
///
/// `--offline`, so the leg cannot pass or fail for a reason that lives on
/// somebody else's server, and `--config` pointing at the tracked `deny.toml`,
/// so what is being watched is the file this repository ships rather than a
/// copy written here that could drift from it.
///
/// The licences and bans legs only. The advisories leg needs the advisory
/// database, which is a network fetch, and this file's header says what that
/// costs.
fn check_fixture(manifest: &Path) -> Output {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let policy = workspace_root().join("deny.toml");

    assert!(
        policy.exists(),
        "there is no deny.toml at {}, so this suite has no policy to watch",
        policy.display()
    );

    std::process::Command::new(cargo)
        .args(["deny", "--offline", "--manifest-path"])
        .arg(manifest)
        .arg("check")
        .arg("--config")
        .arg(&policy)
        .args(["licenses", "bans"])
        .output()
        .unwrap_or_else(|e| {
            panic!(
                "could not run cargo deny: {e}. This target is behind the `policy` \
                 feature because it needs that binary; install it with \
                 `cargo install --locked cargo-deny`."
            )
        })
}

#[test]
fn a_dependency_the_policy_allows_and_no_duplicate_passes() {
    let dir = TempDir::new("policy-control");
    let manifest = write_fixture(dir.path(), LICENCE_ALLOWED, false);

    let out = check_fixture(&manifest);

    assert!(
        out.status.success(),
        "the control leg was refused, so a red in either leg below would not be \
         attributable to the thing that leg changes:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn a_dependency_outside_the_allowed_set_is_refused() {
    let dir = TempDir::new("policy-licence");
    let manifest = write_fixture(dir.path(), LICENCE_REFUSED, false);

    let out = check_fixture(&manifest);
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(
        !out.status.success(),
        "the policy accepted {LICENCE_REFUSED}, which cannot be combined into an \
         AGPL-3.0 work, so the allowed set refuses nothing:\n{stderr}"
    );
    assert!(
        stderr.contains(LICENCE_REFUSED),
        "the policy refused the fixture for a reason that is not the licence, so \
         this leg would stay red if the licence half stopped working:\n{stderr}"
    );
}

#[test]
fn two_versions_of_one_crate_are_refused() {
    let dir = TempDir::new("policy-duplicate");
    let manifest = write_fixture(dir.path(), LICENCE_ALLOWED, true);

    let out = check_fixture(&manifest);
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(
        !out.status.success(),
        "the policy accepted two versions of one crate in one graph:\n{stderr}"
    );
    assert!(
        stderr.contains("duplicate"),
        "the policy refused the fixture for a reason that is not the duplicate, \
         and the only thing changed from the control leg is the second \
         version:\n{stderr}"
    );
}
