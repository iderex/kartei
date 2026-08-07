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

//! A defect on purpose, in code that ships, so the exclusion this change adds
//! can be watched not blinding the analysis to the thing it is for.
//!
//! The change beside this file stops the analysis reading the test targets and
//! the test harness crate. An exclusion is only safe if what is left is still
//! judged, and a reader has no way to tell an exclusion that removed twelve
//! findings about harness code from one that removed the query. This file is
//! how they are told apart: it is in the binary's own crate, which is not
//! excluded, and it carries the defect the query exists for.
//!
//! This is the second of two states this file passes through, and the pull
//! request records the run for each. The first had containment written the way
//! somebody actually writes it wrong, compared before the path was resolved.
//! This one swaps those two lines and changes nothing else, so a green result
//! here is attributable to the defect being gone rather than to the fixture
//! having been rewritten into something the analysis cannot follow. The file is
//! removed in the next commit.
//!
//! The mistake was the order of two lines and nothing else. The path was
//! checked against the directory it is supposed to stay inside, and only then
//! resolved. That is backwards. `..` is still a literal component when the
//! check runs, so a name walking out of the directory passed a check that
//! compared the front of the string, and the resolution afterwards was what
//! turned it into the path that is actually opened.
//!
//! Swapping the two lines is the whole repair, which is why this is the version
//! worth running the check against: an obviously broken input proves the check
//! is wired up, and this one proves it can tell the difference that matters.

use std::path::PathBuf;

/// The data directory an operator gives the server.
const DATA_DIRECTORY: &str = "/var/lib/kartei";

/// Read a file the caller named, from inside the data directory.
pub fn read_from_the_data_directory() -> std::io::Result<Vec<u8>> {
    let name = std::env::args().nth(1).unwrap_or_default();
    let base = PathBuf::from(DATA_DIRECTORY);
    let path = base.join(name).canonicalize()?;

    // After the resolution rather than before it, so the thing compared is the
    // path that gets opened.
    if !path.starts_with(&base) {
        return Err(std::io::Error::other("outside the data directory"));
    }

    std::fs::read(path)
}
