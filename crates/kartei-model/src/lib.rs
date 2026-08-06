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
