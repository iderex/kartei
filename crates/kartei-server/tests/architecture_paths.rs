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

//! The architecture note points at the tree, so a rename must not be allowed to
//! leave it pointing at nothing.
//!
//! The note is written to explain why rather than to restate what, and the way
//! it stays short is by naming files instead of describing them. That trade has
//! one failure: a document full of references rots faster than a document full
//! of prose, because every reference is a thing somebody else can rename, and
//! the rot is invisible until a reader follows a link. This test follows every
//! one of them.
//!
//! Two kinds of reference are read, and each is resolved the way its own reader
//! resolves it.
//!
//! A markdown link target is resolved against the note's own directory, because
//! that is what a browser does with it. A relative link that resolves from the
//! workspace root but not from `docs/` is a broken link on the site and is
//! refused here.
//!
//! A path inside a backtick span is resolved against the workspace root,
//! because that is how a path is written in this repository when it is written
//! for somebody standing in a clone.
//!
//! ## What this does not reach
//!
//! Fenced code blocks are not scanned. They hold commands and their output, and
//! the output of a command is not a claim about a path in this tree, so
//! refusing it would refuse correct transcripts. The cost is real and is stated
//! rather than mitigated: a path that appears only inside a fenced block in the
//! note can go stale and nothing here reports it.
//!
//! Nothing here judges what the note says. A note that names only paths that
//! resolve and is wrong about every one of them passes this test. What the note
//! claims is read by a person in review and nowhere else.

use std::path::{Path, PathBuf};

/// The note, relative to the workspace root.
const NOTE: &str = "docs/architecture.md";

/// Extensions that make a backtick span a path even when it names no directory.
///
/// Without this, a bare file name at the workspace root would be invisible to
/// the check while a path with a slash in it was refused, which is a hole that
/// depends on where a file happens to sit.
const PATH_EXTENSIONS: &[&str] = &[".md", ".rs", ".toml", ".yml", ".yaml", ".lock"];

fn workspace_root() -> PathBuf {
    // crates/kartei-server -> crates -> the workspace root.
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.pop();
    root.pop();
    root
}

/// One reference found in the note, and the base its own reader resolves it
/// against.
#[derive(Debug)]
struct Reference {
    line: usize,
    text: String,
    /// Relative to the note's directory rather than to the workspace root.
    from_note_dir: bool,
}

/// Whether a backtick span is naming a path at all.
///
/// A span holding a flag, a field name or a fragment of a manifest is not, and
/// treating one as a path would make the check refuse correct prose. A span
/// holding whitespace is a command rather than a path.
fn looks_like_a_path(span: &str) -> bool {
    if span.is_empty() || span.chars().any(char::is_whitespace) {
        return false;
    }
    span.contains('/') || PATH_EXTENSIONS.iter().any(|ext| span.ends_with(ext))
}

/// Every reference in the note, in the order they appear.
///
/// Fenced blocks are skipped, for the reason in this file's header.
fn references(note: &str) -> Vec<Reference> {
    let mut found = Vec::new();
    let mut fenced = false;

    for (index, line) in note.lines().enumerate() {
        let number = index + 1;

        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }

        // Markdown link targets: the text between `](` and the matching `)`.
        let mut rest = line;
        while let Some(open) = rest.find("](") {
            rest = &rest[open + 2..];
            let Some(close) = rest.find(')') else { break };
            let target = &rest[..close];
            rest = &rest[close + 1..];

            // Anything with a scheme leaves this tree, and a bare fragment
            // names a heading in the note itself. Neither is a path.
            if target.contains("://") || target.starts_with('#') || target.starts_with("mailto:") {
                continue;
            }
            // A link may carry a heading after the path; the path is the part
            // before it.
            let target = target.split('#').next().unwrap_or(target);
            if target.is_empty() {
                continue;
            }
            found.push(Reference {
                line: number,
                text: target.to_owned(),
                from_note_dir: true,
            });
        }

        // Backtick spans: the odd-numbered pieces of a split on the backtick.
        for (piece, span) in line.split('`').enumerate() {
            if piece % 2 == 1 && looks_like_a_path(span) {
                found.push(Reference {
                    line: number,
                    text: span.to_owned(),
                    from_note_dir: false,
                });
            }
        }
    }

    found
}

fn read_note(root: &Path) -> String {
    std::fs::read_to_string(root.join(NOTE)).unwrap_or_else(|e| {
        panic!("could not read {NOTE}, so there is no note for this check to judge: {e}")
    })
}

#[test]
fn every_path_the_architecture_note_names_resolves_in_the_tree() {
    let root = workspace_root();
    let note = read_note(&root);
    let refs = references(&note);

    // A note that points at nothing has stopped being a note that points, and
    // an empty reference list would otherwise make every assertion below pass
    // by having nothing to check.
    assert!(
        !refs.is_empty(),
        "{NOTE} names no path at all, so either it stopped pointing at the tree \
         or this check stopped finding what it points at"
    );

    let note_dir = root.join(NOTE).parent().unwrap().to_path_buf();
    let mut dead = Vec::new();

    for reference in &refs {
        let base = if reference.from_note_dir {
            &note_dir
        } else {
            &root
        };
        if !base.join(&reference.text).exists() {
            dead.push(format!(
                "{NOTE}:{} names {}, which does not resolve from {}",
                reference.line,
                reference.text,
                if reference.from_note_dir {
                    "the note's own directory"
                } else {
                    "the workspace root"
                }
            ));
        }
    }

    assert!(
        dead.is_empty(),
        "{NOTE} points at {} path(s) and {} of them do not exist:\n  {}",
        refs.len(),
        dead.len(),
        dead.join("\n  ")
    );

    println!(
        "{NOTE}: {} path reference(s) checked, all resolve",
        refs.len()
    );
}

#[test]
fn the_architecture_note_names_no_glob() {
    let root = workspace_root();
    let note = read_note(&root);

    let globs: Vec<String> = references(&note)
        .iter()
        .filter(|r| r.text.contains('*') || r.text.contains('?'))
        .map(|r| format!("{NOTE}:{} names {}", r.line, r.text))
        .collect();

    // A glob would pass the check above by never being looked up, or fail it
    // for being a name no file has. Either way it is a reference nothing can
    // follow, so it is refused here by name rather than left to behave oddly
    // in the other test.
    assert!(
        globs.is_empty(),
        "a glob names no single file, so nothing can check that it still resolves:\n  {}",
        globs.join("\n  ")
    );
}
