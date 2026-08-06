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

//! A guard that fails tells the contributor what to read, and what it tells
//! them to read has to exist.
//!
//! This is not hypothetical here. The DCO check has been in the tree since
//! before there was any source, and its failure message has always ended "See
//! CONTRIBUTING.md and ./DCO." Neither file existed, so the one gate that ran
//! pointed a contributor at nothing, and nothing reported it. #8 is that
//! defect. This test is what stops the next one, because the next one is a
//! rename: a document moves, the message that names it does not move with it,
//! and the gap is invisible until somebody's check goes red and they follow the
//! instruction.
//!
//! ## What is read
//!
//! The workflow annotations, which are the lines a contributor is actually
//! shown: the text after `::error::`, `::warning::` or `::notice::` in any
//! tracked file under `.github/workflows/`.
//!
//! Inside that text, a word is treated as naming a path in this repository when
//! it begins with `./` or ends with `.md`, after quotes and sentence
//! punctuation are trimmed off it. Both shapes are resolved from the workspace
//! root.
//!
//! That rule is deliberately narrow, and the narrowness was measured rather
//! than guessed. A rule that treated any word containing a slash as a path
//! would refuse the tree as it stands today, because the annotations also carry
//! "bidirectional/invisible" and a commit range written with shell
//! interpolation. A check that refuses correct prose gets switched off, so it
//! would protect nothing. Words carrying `$` or `{` are skipped for the same
//! reason: what they name is known when the job runs and not when this test
//! does, so they are not claims about the tree.
//!
//! ## What this does not reach
//!
//! Everything outside an annotation. A path in a comment, in the body of a
//! `run:` step, in a job summary, in an action input or in a workflow's own
//! `name:` is not read here, and can go stale with nothing reporting it. Only
//! the sentence the contributor is shown on a failure is covered.
//!
//! Paths that neither begin with `./` nor end with `.md`, inside an annotation,
//! are also not read. A message naming `.github/workflows/gate.yml` would pass
//! this test whether or not that file existed.
//!
//! Nothing here judges whether the message is good advice. An annotation
//! pointing at a document that exists and says nothing relevant passes.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The markers GitHub reads as an annotation. The text after one of these is
/// what a contributor is shown when the job fails.
const ANNOTATIONS: &[&str] = &["::error::", "::warning::", "::notice::"];

/// Trimmed off the end of a word before it is judged, so that a path at the end
/// of a sentence is the same reference as one in the middle of it. The trailing
/// full stop in "See CONTRIBUTING.md and ./DCO." is exactly the one-character
/// difference this exists for.
const TRIM_END: &[char] = &['.', ',', ';', ':', '"', '\'', ')', '`'];

/// Trimmed off the front, which is a shorter list than the one above and has to
/// be: a full stop is punctuation at the end of a word and the first character
/// of `./DCO` at the front of one. Trimming the same set from both ends turned
/// that reference into `/DCO`, which then matched neither shape and was dropped
/// silently, so the check passed while reading one document instead of two.
const TRIM_START: &[char] = &['"', '\'', '(', '`'];

fn workspace_root() -> PathBuf {
    // crates/kartei-server -> crates -> the workspace root.
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.pop();
    root.pop();
    root
}

/// One path a workflow annotation names, and where it was named.
#[derive(Debug)]
struct Reference {
    file: String,
    line: usize,
    text: String,
}

/// Every tracked workflow file, from git rather than from a directory walk, so
/// that an untracked scratch file in that directory is not judged and a tracked
/// one can never be missed.
fn tracked_workflows(root: &Path) -> Vec<(String, String)> {
    let out = Command::new("git")
        .args(["ls-files", "-z", ".github/workflows"])
        .current_dir(root)
        .output()
        .unwrap_or_else(|e| panic!("could not run git ls-files: {e}"));

    assert!(
        out.status.success(),
        "git ls-files failed, so the workflow list is unknown rather than empty:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let files: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .split('\0')
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();

    assert!(
        !files.is_empty(),
        "no tracked workflow was found, which is a broken query rather than a repository \
         with no gate"
    );

    files
        .into_iter()
        .map(|name| {
            let text = std::fs::read_to_string(root.join(&name))
                .unwrap_or_else(|e| panic!("could not read {name}: {e}"));
            (name, text)
        })
        .collect()
}

/// Whether a word, already trimmed, is naming a path in this repository.
fn names_a_path(word: &str) -> bool {
    if word.is_empty() || word.contains('$') || word.contains('{') {
        return false;
    }
    word.starts_with("./") || word.ends_with(".md")
}

/// Every path named by an annotation in one file.
fn references(name: &str, text: &str) -> Vec<Reference> {
    let mut found = Vec::new();

    for (index, line) in text.lines().enumerate() {
        let Some(message) = ANNOTATIONS
            .iter()
            .find_map(|marker| line.split_once(marker).map(|(_, rest)| rest))
        else {
            continue;
        };

        for word in message.split_whitespace() {
            let word = word
                .trim_start_matches(TRIM_START)
                .trim_end_matches(TRIM_END);
            if names_a_path(word) {
                found.push(Reference {
                    file: name.to_owned(),
                    line: index + 1,
                    text: word.to_owned(),
                });
            }
        }
    }

    found
}

fn all_references(root: &Path) -> Vec<Reference> {
    tracked_workflows(root)
        .iter()
        .flat_map(|(name, text)| references(name, text))
        .collect()
}

#[test]
fn every_document_a_workflow_annotation_names_resolves_in_the_tree() {
    let root = workspace_root();
    let refs = all_references(&root);

    // An empty list is the failure mode this test has of its own: a change to
    // the reading above that stops finding anything would leave every assertion
    // below passing with nothing behind it. The cost is that removing the last
    // annotation that names a document reds this test, and the repair for that
    // is to delete this test rather than to leave it asserting a reference set
    // that no longer exists.
    assert!(
        !refs.is_empty(),
        "no workflow annotation names a document, so either the gate stopped telling \
         contributors what to read or this check stopped finding what it names"
    );

    let dead: Vec<String> = refs
        .iter()
        .filter(|r| !root.join(&r.text).exists())
        .map(|r| {
            format!(
                "{}:{} names {}, which is not in the tree",
                r.file, r.line, r.text
            )
        })
        .collect();

    assert!(
        dead.is_empty(),
        "of the {} path(s) named in a workflow annotation, {} cannot be followed, \
         so a contributor reading that message is sent to nothing:\n  {}",
        refs.len(),
        dead.len(),
        dead.join("\n  ")
    );

    println!(
        "{} document reference(s) in workflow annotations checked, all resolve",
        refs.len()
    );
}

#[test]
fn the_dco_check_still_names_the_two_documents_it_asserts_agreement_with() {
    let root = workspace_root();

    // The DCO check asserts that an author agreed to a text. That assertion is
    // only meaningful if the text is readable, so these two references are not
    // merely required to resolve when they are present: they are required to be
    // present. Dropping them from the message would pass the test above by
    // having nothing left to check.
    let named: Vec<String> = all_references(&root)
        .into_iter()
        .filter(|r| r.file == ".github/workflows/dco.yml")
        .map(|r| r.text)
        .collect();

    for wanted in ["CONTRIBUTING.md", "./DCO"] {
        assert!(
            named.iter().any(|n| n == wanted),
            "the DCO check no longer points a failing contributor at {wanted}; it names {named:?}"
        );
    }
}
