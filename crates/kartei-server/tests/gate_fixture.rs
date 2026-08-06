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

//! The defect the `test` check names, and nothing else.
//!
//! `test` runs the suite. The defect it names is a test that does not hold, so
//! the assertion below is false at run time and true at compile time: nothing
//! here warns, nothing here lints, and rustfmt has no objection. Only the leg
//! that runs the code can see it.

/// Adds two and two, behind a call so the comparison is made at run time.
fn sum() -> i32 {
    2 + 2
}

#[test]
fn the_fixture_fails_where_only_the_suite_can_see_it() {
    assert_eq!(sum(), 5, "two and two are not five, which is the point");
}
