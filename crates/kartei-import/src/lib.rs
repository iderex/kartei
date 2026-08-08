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

//! The importers that bring an existing workspace in.
//!
//! An importer reads an export and produces model data. It depends on the
//! model and on nothing else in the workspace, which keeps two properties: an
//! importer cannot write to storage behind the apply path, and an importer can
//! be run against a fixture without a database or a replicated document
//! anywhere in the test.
//!
//! The importers themselves land with the migration milestone. [`csv`] is the
//! first of them, and it is also the building block the others read their
//! tables through, so it lands ahead of the exports that will use it.

pub mod csv;
