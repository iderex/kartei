//! The binary: HTTP, sockets, configuration, and the assembly of the crates
//! that do the work.
//!
//! This is the only crate that depends on every other one, because it is the
//! only crate that is allowed to know how they fit together. It is also where
//! an engine is selected, by enabling the `engine` feature on `kartei-sync`.
//!
//! Nothing is assembled yet. The parts land with their own milestones, and the
//! layout exists first so that each one has a place to land in and a dependency
//! graph that can be checked before it does.

fn main() {}
