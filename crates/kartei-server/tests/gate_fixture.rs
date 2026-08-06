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

//! The defect the `build` check names, and nothing else.
//!
//! `build` compiles the workspace with warnings denied. The defect it names is
//! a warning, so the file carries exactly one: a binding nothing reads. It
//! compiles, it is formatted, and its test passes, so the only leg that has a
//! reason to move is the one under test.
//!
//! This file is removed again before the branch lands. It exists so the check
//! is required on the strength of a run rather than of a description.

/// Passes, and leaves behind a binding the compiler can see is never read.
#[test]
fn the_fixture_passes_and_warns() {
    let unread = 4;
    assert_eq!(2 + 2, 4);
}
