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

//! A ruleset requires a check by its literal name, so one name on a head has to
//! mean one verdict.
//!
//! A workflow that triggers on `pull_request` and also on a push to the branch
//! the pull request is opened from runs twice for the same commit, and each run
//! publishes a check run under the same job name. The head then carries two
//! check runs called the same thing, from the same app, and which of them a
//! required name is answered by is not something this repository decides.
//!
//! That state was in the tree. `unicode-guard.yml` triggered on
//! `push: branches: ["**"]` beside its `pull_request` trigger, and every pull
//! request head produced two check runs named `Reject Trojan Source Unicode`.
//! #6 is where that was found and this is what stops it coming back.
//!
//! ## The rule
//!
//! A workflow that triggers on a pull request may trigger on a push only for
//! `main`. Every other workflow in the tree was already written that way; the
//! one that was not is the one that produced the duplicate.
//!
//! A push trigger with no `branches:` filter is refused too, and that is the
//! version worth having. Leaving the filter out is the shorter thing to write
//! and it means every branch, so the defect arrives as an absence rather than
//! as a wrong value.
//!
//! ## What this does not reach
//!
//! It reads the trigger and never a run. Two workflows that give two different
//! jobs the same `name:` would produce the same collision and are not judged
//! here, because that is a comparison across files that nothing in this tree
//! needs yet.
//!
//! It treats `main` as a branch no pull request is opened from. That holds
//! today: `main` is the base of every pull request here and the ruleset refuses
//! a direct push to it. A pull request opened from `main` towards some other
//! branch would produce the duplicate this test is about and pass it.
//!
//! It reads the trigger block by indentation rather than with a YAML parser.
//! The workspace carries no third-party dependency and a parser is not worth
//! becoming the first one for five files that are all written in the same
//! shape. The cost is paid where it belongs: a file whose `on:` block cannot be
//! read is refused rather than skipped, so a shape this reader does not
//! understand reds this test instead of passing it.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The trigger names that mean "this ran because of a pull request".
///
/// `pull_request_target` is here although no workflow in the tree uses one. It
/// produces a check run on the same head as `pull_request` does, so a workflow
/// that gained one would gain the duplicate without this test noticing.
const PULL_REQUEST_TRIGGERS: &[&str] = &["pull_request", "pull_request_target"];

/// The one branch a push may trigger on beside a pull request, because no pull
/// request in this repository is opened from it.
const PUSH_BRANCH_ALLOWED: &str = "main";

fn workspace_root() -> PathBuf {
    // crates/kartei-server -> crates -> the workspace root.
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.pop();
    root.pop();
    root
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

/// One trigger declared in a workflow's `on:` block.
#[derive(Debug)]
struct Trigger {
    name: String,
    /// The `branches:` filter, or `None` where the trigger declares no filter
    /// and therefore matches every branch.
    branches: Option<Vec<String>>,
}

/// Whether a line is a comment or blank, which carries no structure either way.
fn is_blank_or_comment(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.is_empty() || trimmed.starts_with('#')
}

/// The number of leading spaces on a line.
fn indent(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

/// The lines of the top-level `on:` block, without the `on:` line itself.
///
/// `None` where the file declares no such block, which is a shape this reader
/// does not understand rather than a workflow with no trigger.
fn on_block(text: &str) -> Option<Vec<&str>> {
    let mut lines = text.lines();
    lines.find(|line| line.trim_end() == "on:" && indent(line) == 0)?;

    let mut block = Vec::new();
    for line in lines {
        if is_blank_or_comment(line) {
            continue;
        }
        if indent(line) == 0 {
            break;
        }
        block.push(line);
    }
    Some(block)
}

/// Parse a `branches:` value, in either of the two shapes YAML allows here.
///
/// An inline `[a, b]` is read from the same line; a block list is read from the
/// lines under it. A quoted entry is unquoted, so `"**"` and `**` are one value.
fn branch_list(value: &str, following: &[&str], value_indent: usize) -> Vec<String> {
    let unquote = |s: &str| s.trim().trim_matches(['"', '\'']).to_owned();

    let value = value.trim();
    if let Some(inner) = value.strip_prefix('[').and_then(|v| v.strip_suffix(']')) {
        return inner
            .split(',')
            .map(unquote)
            .filter(|s| !s.is_empty())
            .collect();
    }

    following
        .iter()
        .take_while(|line| indent(line) > value_indent)
        .filter_map(|line| line.trim().strip_prefix("- "))
        .map(unquote)
        .collect()
}

/// Every trigger a workflow declares, with its branch filter.
fn triggers(name: &str, text: &str) -> Vec<Trigger> {
    let block = on_block(text).unwrap_or_else(|| {
        panic!(
            "{name} declares no top-level `on:` block that this check can read, so its \
             triggers are unknown rather than absent"
        )
    });

    // The `on:` block's own keys sit one level in. Reading the level from the
    // first key rather than assuming two spaces keeps the check working on a
    // file indented differently, and a file mixing levels reds below.
    let Some(key_indent) = block.first().map(|line| indent(line)) else {
        return Vec::new();
    };

    let mut found = Vec::new();
    for (index, line) in block.iter().enumerate() {
        if indent(line) != key_indent {
            continue;
        }
        let Some((key, _)) = line.trim().split_once(':') else {
            continue;
        };

        let rest = &block[index + 1..];
        let branches = rest
            .iter()
            .take_while(|following| indent(following) > key_indent)
            .enumerate()
            .find_map(|(offset, following)| {
                following
                    .trim()
                    .strip_prefix("branches:")
                    .map(|value| branch_list(value, &rest[offset + 1..], indent(following)))
            });

        found.push(Trigger {
            name: key.trim().to_owned(),
            branches,
        });
    }

    found
}

#[test]
fn no_workflow_produces_one_check_run_name_twice_on_one_head() {
    let root = workspace_root();
    let workflows = tracked_workflows(&root);

    let mut judged = 0;
    let mut duplicates = Vec::new();

    for (name, text) in &workflows {
        let declared = triggers(name, text);
        if !declared
            .iter()
            .any(|t| PULL_REQUEST_TRIGGERS.contains(&t.name.as_str()))
        {
            continue;
        }
        judged += 1;

        let Some(push) = declared.iter().find(|t| t.name == "push") else {
            continue;
        };

        match &push.branches {
            None => duplicates.push(format!(
                "{name} triggers on a pull request and on a push with no branches filter, \
                 so a push to the branch a pull request is open on runs it a second time"
            )),
            Some(branches) => {
                let wider: Vec<&str> = branches
                    .iter()
                    .map(String::as_str)
                    .filter(|b| *b != PUSH_BRANCH_ALLOWED)
                    .collect();
                if !wider.is_empty() {
                    duplicates.push(format!(
                        "{name} triggers on a pull request and on a push to {}, \
                         so a push to the branch a pull request is open on runs it a second time",
                        wider.join(", ")
                    ));
                }
            }
        }
    }

    // The empty-set failure this check has of its own. A change to the reading
    // above that stopped finding any pull-request trigger would leave the
    // assertion below passing with nothing behind it.
    assert!(
        judged > 0,
        "no tracked workflow declares a pull request trigger, so either the gate stopped \
         running on pull requests or this check stopped reading triggers"
    );

    assert!(
        duplicates.is_empty(),
        "{} of the {} workflow(s) that run on a pull request also run on a push to the same \
         branch, so their check-run name appears twice on one head and a ruleset requiring \
         that name has two verdicts to choose between:\n  {}",
        duplicates.len(),
        judged,
        duplicates.join("\n  ")
    );

    println!(
        "{judged} workflow(s) run on a pull request; none of them also runs on a push to a \
         branch a pull request can be open on"
    );
}
