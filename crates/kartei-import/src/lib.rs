//! The importers that bring an existing workspace in.
//!
//! An importer reads an export and produces model data. It depends on the
//! model and on nothing else in the workspace, which keeps two properties: an
//! importer cannot write to storage behind the apply path, and an importer can
//! be run against a fixture without a database or a replicated document
//! anywhere in the test.
//!
//! The importers themselves land with the migration milestone.
