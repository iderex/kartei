//! The wire messages between replicas, and their versioning.
//!
//! The frames are defined here and nowhere else, so both ends of a connection
//! read one definition. The crate names no engine and no storage type: a frame
//! carrying a document update carries bytes and a version, and what produced
//! those bytes is the sync crate's business.
//!
//! The frames and the version negotiation land with the transport milestone.
