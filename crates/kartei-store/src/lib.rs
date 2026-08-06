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

//! The embedded database: the schema, the migrations and the relational
//! projection that the grid, board and calendar views read.
//!
//! It depends on the model and on the replicated document trait. It does not
//! depend on a replication engine, and it must not: the projection is derived
//! from a document, and which engine produced that document is not something
//! this crate is allowed to know.
//!
//! The schema, the migrations and the apply path land with the storage
//! milestone.
