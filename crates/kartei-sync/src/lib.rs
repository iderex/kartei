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

//! The replicated document, behind one trait.
//!
//! This is the only crate in the workspace that is allowed to name a
//! replication engine, and it may name one only behind the `engine` feature.
//! Everything above the boundary talks about documents, updates and versions,
//! which are this crate's own types, so the engine can be swapped without a
//! change above it.
//!
//! The boundary is not a convenience. The engine choice is the highest
//! consequence decision in the project and has the least evidence behind it,
//! so the layout assumes it may turn out wrong and keeps the cost of being
//! wrong to one crate.
//!
//! The trait, the update and version types and the conformance suite that runs
//! against the trait land with the sync milestone. What exists here today is
//! the crate and the empty feature slot, so the crates above it can be built
//! and their dependency graphs can be checked before an engine exists.
