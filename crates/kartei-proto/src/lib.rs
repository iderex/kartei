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

//! The wire messages between replicas, and their versioning.
//!
//! The frames are defined here and nowhere else, so both ends of a connection
//! read one definition. The crate names no engine and no storage type: a frame
//! carrying a document update carries bytes and a version, and what produced
//! those bytes is the sync crate's business.
//!
//! The frames and the version negotiation land with the transport milestone.
