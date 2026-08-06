//! The GPU suite. It is empty, and it exists so that the first test that needs
//! hardware acceleration has somewhere to go that is not the main suite.
//!
//! Nothing here runs in the gate. The target is behind the `gpu` feature, which
//! `cargo test --workspace` does not enable, so this file is neither built nor
//! run there. Run it deliberately:
//!
//!     cargo test -p kartei-testing --features gpu --test gpu
//!
//! What a test in this file has to do, when there is one. Detect the absence of
//! the hardware it needs and skip with a printed reason, rather than passing
//! quietly. A GPU test that passes on a machine with no GPU is a test that says
//! nothing and reads as if it said something, and that is the failure this
//! separation exists to prevent. Rust's test harness has no skip verdict, so
//! the skip is a printed line and an early return, and the line names what was
//! missing.
//!
//! The opposite direction is a rule too. No test outside this file may require
//! the absence of a GPU, so a contributor's machine that has one does not fail
//! a suite that a build machine passes.
