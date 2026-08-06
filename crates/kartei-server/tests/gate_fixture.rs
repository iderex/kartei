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

//! The defect the `lint` check names, and nothing else.
//!
//! `lint` runs clippy with warnings denied. The defect it names is a lint the
//! compiler does not have, so the file carries a `return` on the last
//! expression of a function: `clippy::needless_return` is warn by default and
//! rustc says nothing about it. The function is called, so it is not dead code,
//! and the file compiles, formats and passes.

/// Upper-cases a word, and returns it the one way clippy objects to.
fn shout(word: &str) -> String {
    return word.to_uppercase();
}

#[test]
fn the_fixture_passes_and_is_linted() {
    assert_eq!(shout("gate"), "GATE");
}
