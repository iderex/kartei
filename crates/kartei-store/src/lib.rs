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
