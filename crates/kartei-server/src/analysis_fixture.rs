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
//! This file exists for one commit. The commit after it removes the file and
//! the call below, and the two runs are what the pull request records: red with
//! this here, green with it gone, so the failure is attributable to the defect
//! rather than to the check having been wired up wrong.
//!
//! The defect is the one somebody actually writes. A name arrives from outside
//! the process, it is joined onto a directory the program owns, and the result
//! is opened. Joining looks like it confines the read to that directory and it
//! does not: a name holding `..` walks out of it, and on this path a name is
//! whatever the caller passed.

use std::path::PathBuf;

/// The data directory an operator gives the server.
const DATA_DIRECTORY: &str = "/var/lib/kartei";

/// Read a file the caller named, from inside the data directory.
pub fn read_from_the_data_directory() -> std::io::Result<Vec<u8>> {
    let name = std::env::args().nth(1).unwrap_or_default();
    let path = PathBuf::from(DATA_DIRECTORY).join(name);
    std::fs::read(path)
}
