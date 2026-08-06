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

//! The defect the `format` check names, and nothing else.
//!
//! `format` reports and does not rewrite. The defect it names is source that
//! rustfmt would lay out differently, so the body below is indented eight
//! spaces where rustfmt writes four. Nothing here is a compiler or clippy
//! finding: indentation is not a lint, and the test passes.

/// The number the test below expects, behind a call so no constant is folded.
fn four() -> i32 {
    4
}

#[test]
fn the_fixture_passes_and_is_misformatted() {
        assert_eq!(four(), 4);
}
