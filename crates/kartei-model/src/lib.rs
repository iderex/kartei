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

//! The block schema and the field schema, as plain data.
//!
//! This crate knows nothing about storage, about sync or about the wire. It
//! holds the shapes that the editor, the relational views, the importers and
//! the storage projection all agree on, and it holds them as ordinary types
//! that can be constructed and compared without a runtime.
//!
//! The boundary is the reason the crate exists. Everything else in the
//! workspace may depend on the model; the model depends on nothing, so a change
//! to how documents are replicated or stored cannot reach it.
//!
//! The types themselves are not defined here yet. The block schema is settled
//! in the milestone that defines it and lands with that work.
