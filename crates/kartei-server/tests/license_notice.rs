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

//! Every tracked Rust source file carries the notice the licence asks for.
//!
//! The AGPL's own appendix says to attach the notice to the start of each source
//! file, and that each file should carry at least the copyright line and a
//! pointer to where the full notice is found. A file added later without one is
//! a file whose exclusion of warranty is stated nowhere a reader of that file
//! will see, and nothing announces it, so this test refuses it.
//!
//! The expected text is read out of `LICENSE` rather than written here. A copy
//! in this file would be a second version of the notice that drifts from the
//! first, and the drift would be invisible precisely because both look right on
//! their own.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The first line of the appendix notice in `LICENSE`, indented as it is there.
const NOTICE_FIRST: &str = "    kartei, a self-hosted workspace for documents";
/// The last line of it.
const NOTICE_LAST: &str =
    "    along with this program.  If not, see <https://www.gnu.org/licenses/>.";

fn workspace_root() -> PathBuf {
    // crates/kartei-server -> crates -> the workspace root.
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.pop();
    root.pop();
    root
}

/// The notice the licence asks for, as it would appear at the top of a Rust
/// file: the appendix block with its indent removed and each line commented.
///
/// Lines rather than one string, because a clone with `core.autocrlf` set holds
/// the working copy with carriage returns and a comparison of raw bytes would
/// then refuse a correct file on one machine and accept it on another. `lines`
/// drops the carriage return on both sides, so the comparison is of text.
fn expected_notice(root: &Path) -> Vec<String> {
    let licence = std::fs::read_to_string(root.join("LICENSE")).unwrap_or_else(|e| {
        panic!("could not read LICENSE, so there is nothing to compare against: {e}")
    });

    let mut lines = licence.lines().skip_while(|l| !l.starts_with(NOTICE_FIRST));

    let mut block = Vec::new();
    for line in &mut lines {
        block.push(line);
        if line == NOTICE_LAST {
            break;
        }
    }

    assert!(
        block.last() == Some(&NOTICE_LAST),
        "the appendix notice was not found in LICENSE between its first line and its last, \
         so this test cannot say what a file should carry"
    );

    block
        .iter()
        .map(|line| {
            let text = line.strip_prefix("    ").unwrap_or(line);
            if text.is_empty() {
                "//".to_owned()
            } else {
                format!("// {text}")
            }
        })
        .collect()
}

/// Every tracked Rust file, from git rather than from a directory walk: a walk
/// would reach build output, and what the licence is about is what is
/// distributed.
fn tracked_rust_files(root: &Path) -> Vec<PathBuf> {
    let out = Command::new("git")
        .args(["ls-files", "-z", "*.rs"])
        .current_dir(root)
        .output()
        .unwrap_or_else(|e| panic!("could not run git ls-files: {e}"));

    assert!(
        out.status.success(),
        "git ls-files failed, so the file list is unknown rather than empty:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let files: Vec<PathBuf> = String::from_utf8_lossy(&out.stdout)
        .split('\0')
        .filter(|s| !s.is_empty())
        .map(|s| root.join(s))
        .collect();

    assert!(
        !files.is_empty(),
        "no tracked Rust file was found, which is a broken query rather than a clean tree"
    );

    files
}

#[test]
fn every_tracked_rust_file_opens_with_the_notice_the_licence_asks_for() {
    let root = workspace_root();
    let expected = expected_notice(&root);

    let mut missing = Vec::new();

    for file in tracked_rust_files(&root) {
        let text = std::fs::read_to_string(&file)
            .unwrap_or_else(|e| panic!("could not read {}: {e}", file.display()));

        let head: Vec<&str> = text.lines().take(expected.len()).collect();

        if head != expected {
            missing.push(
                file.strip_prefix(&root)
                    .unwrap_or(&file)
                    .display()
                    .to_string(),
            );
        }
    }

    assert!(
        missing.is_empty(),
        "these files do not open with the notice in LICENSE:\n  {}",
        missing.join("\n  ")
    );
}
