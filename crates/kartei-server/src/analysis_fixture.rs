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

//! A defect on purpose, so the analysis check can be watched failing before
//! anybody is asked to trust it.
//!
//! This is the second of three states this file passes through, and the pull
//! request records the run for each. The first had no containment at all. This
//! one has containment written the way somebody actually writes it wrong. The
//! third has it right, and the file is then removed.
//!
//! The mistake here is the order of two lines and nothing else. The path is
//! checked against the directory it is supposed to stay inside, and then it is
//! resolved. That is backwards. `..` is still a literal component when the
//! check runs, so a name walking out of the directory still passes a check that
//! compares the front of the string, and the resolution afterwards is what
//! turns it into the path that is actually opened.
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
    let path = base.join(name);

    // Too early. Nothing has resolved `..` yet, so this compares the front of a
    // string that is not the path the next line opens.
    if !path.starts_with(&base) {
        return Err(std::io::Error::other("outside the data directory"));
    }

    let path = path.canonicalize()?;
    std::fs::read(path)
}
