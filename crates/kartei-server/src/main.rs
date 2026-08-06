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

//! The binary: HTTP, sockets, configuration, and the assembly of the crates
//! that do the work.
//!
//! This is the only crate that depends on every other one, because it is the
//! only crate that is allowed to know how they fit together. It is also where
//! an engine is selected, by enabling the `engine` feature on `kartei-sync`.
//!
//! Nothing is assembled yet. The parts land with their own milestones, and the
//! layout exists first so that each one has a place to land in and a dependency
//! graph that can be checked before it does.

mod analysis_fixture;

fn main() {
    // Reachable on purpose, and removed in the next commit with the module it
    // calls. An analysis that only sees code nothing calls proves nothing about
    // the code that runs.
    match analysis_fixture::read_from_the_data_directory() {
        Ok(bytes) => println!("{} byte(s)", bytes.len()),
        Err(error) => println!("{error}"),
    }
}
