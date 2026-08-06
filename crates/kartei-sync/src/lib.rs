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
