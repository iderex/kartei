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
//! This is the third of three states this file passes through, and the pull
//! request records the run for each. The first had no containment at all and
//! the check reddened. The second had containment in the wrong order, which is
//! how the mistake is really written, and the check reddened on that too. This
//! one has the same containment in the right order, and it is the state the
//! check is expected to pass.
//!
//! What separates this from the state before it is which side of the resolution
//! the comparison sits on. `..` is a literal component until the path is
//! resolved, so a comparison ahead of the resolution is made against something
//! other than the path that gets opened. Behind it, the comparison is made
//! against the real one.
//!
//! Nothing else about the file moved between the two, which is what makes the
//! green result attributable to the order rather than to the fixture having
//! been rewritten. The commit after this one removes the file and its caller.

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
