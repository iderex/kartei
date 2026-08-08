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

//! The importer has no way to reach the network, and something refuses one
//! being added.
//!
//! `kartei-import` is the one importer that is meant to be entirely local, and
//! a sentence saying so is not what keeps it that way. Somebody extending the
//! markdown importer to fetch a referenced image over a link, in good faith,
//! is how it stops being true, and the fetch works, so nothing complains.
//!
//! A test that ran an import and asserted no connection was made would pass
//! just as well against an importer that never had the means to make one, and
//! that is the failure mode a negative assertion always has. So this suite
//! refuses the means instead, at the two places a means can enter the crate.
//!
//! ## The two ways in
//!
//! A crate. Everything that speaks a network protocol in this language arrives
//! as a dependency, so the first leg reads the importer's real dependency
//! graph and refuses a crate nobody has judged. The list below is not a list of
//! crates that are banned, which would be empty today and would therefore
//! assert nothing; it is the list of crates that are allowed, each with the
//! reason it opens no socket. A crate added to the importer reds this leg until
//! somebody writes that reason down, and writing it down is the argument.
//!
//! The standard library, which is on no dependency graph. `std::net` opens a
//! socket with no crate at all, `std::process` launches a program that can open
//! one where the graph cannot see it, and `unsafe` reaches the platform's own
//! socket call past both. So the second leg reads the crate's source.
//!
//! ## Why this judges the importer from outside it
//!
//! `#![forbid(unsafe_code)]` in the importer's own root would refuse the third
//! of those at compile time, which is stronger than reading text. It is also
//! deletable by the same edit that adds the `unsafe` block, and one line
//! removed from a file is not a thing a reviewer reliably sees next to the
//! change that needed it removed. A suite in another crate cannot be switched
//! off from inside the crate it judges. The attribute is worth having as well
//! and is named here so that adding it later is not read as making this
//! redundant.
//!
//! ## The near-miss
//!
//! The mistake this is pointed at is a small HTTP client added to fetch a
//! linked image during an import, because the markdown importer's scope already
//! says referenced images are imported from the tree and the difference between
//! a tree and a link is one branch in a function. A fixture holding an obvious
//! back channel would red just as reliably and would prove much less.
//!
//! Build edges are read alongside normal ones. A build script runs on the
//! machine doing the build, so a crate reached only from one is still code this
//! repository causes to run, and it is the edge somebody looking at a
//! dependency list does not look at.
//!
//! ## What this does not reach
//!
//! It refuses the capability and never observes the behaviour. Nothing here
//! opens a socket, watches for one, or takes the network away from the importer
//! and sees what it does. Taking the network away means changing the machine's
//! configuration, which needs rights a test may not take, so the property is
//! held at the point the capability enters the crate rather than at the point
//! it would be used. An importer that had a socket and did not use it would
//! fail this suite, and an importer that reached the network by a route neither
//! leg reads would pass it.
//!
//! Development dependencies are outside the first leg, which reads normal and
//! build edges only. A dev-dependency is a dependency of the tests rather than
//! of the artefact and reaches no shipped byte. The second leg does read the
//! test sources, because an importer whose own fixtures fetch something is not
//! an importer that was demonstrated offline.
//!
//! A judged crate that grows a socket in a later version is not caught. The
//! list records a judgement about a name, and the version it was judged at is
//! in `Cargo.lock` rather than here.
//!
//! The source leg matches text, so a network surface reached through a macro,
//! or re-exported from a judged crate under another name, is invisible to it.
//! It also drops everything after the first `//` on a line before matching, so
//! a forbidden token written after a `//` inside a string literal on the same
//! line is not seen. Both are the cost of not carrying a parser here, and the
//! first leg is what stands behind them: neither shape arrives without a crate.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The crate under judgement.
const IMPORTER: &str = "kartei-import";

/// Its directory, relative to the workspace root.
const IMPORTER_DIR: &str = "crates/kartei-import";

/// Every crate allowed on the importer's normal and build graph, with the
/// reason it cannot open a socket.
///
/// Both directions fail. A crate on the graph and not here is unjudged, and a
/// crate here and not on the graph is a judgement about nothing, which is how a
/// list like this rots into a place to put names.
const JUDGED: &[(&str, &str)] = &[
    (
        "kartei-import",
        "the crate under judgement; the source leg reads it",
    ),
    (
        "kartei-model",
        "plain data with no dependency of its own, so it carries no client and \
         cannot acquire one without reddening this leg through its own name",
    ),
];

/// What the standard library offers that no dependency graph shows, and what
/// each one would let the importer do.
///
/// Word boundaries are respected, so `unsafe_code` in an attribute is not a
/// match for `unsafe` and a crate named after one of these is not a match
/// either.
const FORBIDDEN_TOKENS: &[(&str, &str)] = &[
    (
        "std::net",
        "opens a socket with no dependency for the first leg to see",
    ),
    (
        "std::process",
        "launches a program that can open one on the importer's behalf",
    ),
    (
        "unsafe",
        "reaches the platform's own socket call past both other legs",
    ),
];

fn workspace_root() -> PathBuf {
    // crates/kartei-server -> crates -> the workspace root.
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.pop();
    root.pop();
    root
}

/// Every crate on `package`'s normal and build graph, including `package`.
fn shipped_dependencies(package: &str) -> BTreeSet<String> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let mut manifest = workspace_root();
    manifest.push("Cargo.toml");

    let out = Command::new(cargo)
        .args([
            "tree",
            "--locked",
            "--edges",
            "normal,build",
            "--prefix",
            "none",
        ])
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
        // A repeated subtree is printed as `(*)` under a bare name, which the
        // first-token read already collapses onto the name itself.
        .filter(|name| *name != "(*)")
        .map(str::to_owned)
        .collect()
}

/// Every `.rs` file under `dir`, in a stable order.
fn rust_sources(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];

    while let Some(next) = pending.pop() {
        let entries = std::fs::read_dir(&next)
            .unwrap_or_else(|e| panic!("could not read {}: {e}", next.display()));

        for entry in entries {
            let entry = entry.expect("could not read a directory entry");
            let path = entry.path();

            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                found.push(path);
            }
        }
    }

    found.sort();
    found
}

/// Whether `haystack` contains `needle` as a token rather than as part of a
/// longer identifier.
fn contains_token(haystack: &str, needle: &str) -> bool {
    let identifier = |c: char| c.is_alphanumeric() || c == '_';
    let mut from = 0;

    while let Some(at) = haystack[from..].find(needle) {
        let start = from + at;
        let end = start + needle.len();

        let before_is_identifier = haystack[..start]
            .chars()
            .next_back()
            .is_some_and(identifier);
        let after_is_identifier = haystack[end..].chars().next().is_some_and(identifier);

        if !before_is_identifier && !after_is_identifier {
            return true;
        }

        from = end;
    }

    false
}

#[test]
fn every_crate_beneath_the_importer_has_been_judged_to_open_no_socket() {
    let on_graph = shipped_dependencies(IMPORTER);
    let judged: BTreeSet<String> = JUDGED.iter().map(|(name, _)| (*name).to_owned()).collect();

    let unjudged: Vec<&String> = on_graph.difference(&judged).collect();
    let stale: Vec<&String> = judged.difference(&on_graph).collect();

    assert!(
        unjudged.is_empty(),
        "{IMPORTER} depends on {} crate(s) nobody has judged to open no socket. \
         Add each to JUDGED with the reason it cannot, or take the dependency \
         out:\n  {}",
        unjudged.len(),
        unjudged
            .iter()
            .map(|n| n.as_str())
            .collect::<Vec<_>>()
            .join("\n  ")
    );

    assert!(
        stale.is_empty(),
        "JUDGED names {} crate(s) that are not on {IMPORTER}'s graph, so the \
         reason recorded beside each is a judgement about nothing:\n  {}",
        stale.len(),
        stale
            .iter()
            .map(|n| n.as_str())
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}

#[test]
fn the_importer_source_reaches_the_network_through_nothing_the_graph_cannot_show() {
    let root = workspace_root();
    let dir = root.join(IMPORTER_DIR);
    let sources = rust_sources(&dir);

    assert!(
        !sources.is_empty(),
        "found no Rust source under {}, so this leg read nothing",
        dir.display()
    );

    let mut found = Vec::new();

    for path in &sources {
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("could not read {}: {e}", path.display()));

        for (number, line) in text.lines().enumerate() {
            // Comments carry prose about these tokens, including the prose in
            // this repository's own doc comments, so a match inside one is not
            // a use.
            let code = match line.find("//") {
                Some(at) => &line[..at],
                None => line,
            };

            for (token, reason) in FORBIDDEN_TOKENS {
                if contains_token(code, token) {
                    let shown = path.strip_prefix(&root).unwrap_or(path);
                    found.push(format!(
                        "{}:{}: {token}, which {reason}",
                        shown.display().to_string().replace('\\', "/"),
                        number + 1
                    ));
                }
            }
        }
    }

    assert!(
        found.is_empty(),
        "the importer names {} way(s) to reach the network that its dependency \
         graph cannot show:\n  {}",
        found.len(),
        found.join("\n  ")
    );
}
