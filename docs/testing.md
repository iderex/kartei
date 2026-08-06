# Testing

These conditions hold from the first test. They are cheap to keep and expensive
to recover: none of them is hard to fix in one test, and all of them are hard to
fix once a hundred tests assume the environment. #7 records the two public
failures on a neighbouring project that this is derived from.

## The rules

A test does not require elevation. No administrator or root rights, no symlink
creation on Windows, no privileged port, no write outside a directory the test
itself created, no service installation, no registry or system configuration
change.

A test does not require a display. No windowing system, no `DISPLAY`, no desktop
session.

A test does not require a GPU, and does not require the absence of one. Anything
that touches hardware acceleration goes in the GPU suite, which is named for
what it is, is not part of the gate, and skips with a printed reason rather than
passing quietly when the hardware is not there.

A test gets its own temporary directory, created by the test and removed by it,
never a shared well known path.

A test does not bind a fixed port. It binds zero and reads back what was
assigned.

A test does not reach the network. The transport suites run against an
in process server.

That is six rules. #7 asks for five in its Done-when and lists six above it; all
six are written here, and the count in that sentence is the thing that is wrong
rather than one of the rules being dropped.

## What the harness gives you

`kartei-testing` is a dev-dependency, never a normal one. It has no
dependencies of its own.

`TempDir::new(label)` creates a directory that belongs to one test and removes
it when the value is dropped. The name carries the process id, a nanosecond
timestamp and a counter, and the directory is created with `create_dir`, which
refuses an existing name rather than sharing it, so a collision becomes another
attempt instead of two tests in one directory.

`bind_ephemeral()` binds port zero on the loopback interface and returns the
bound listener. `port_of(&listener)` reads the port back. The listener is
returned rather than a bare number on purpose: reading a port and closing the
socket hands out a number another process can take first, which is the fixed
port failure again with a longer fuse.

`is_elevated()` asks the operating system whether this process has
administrative rights, and `assert_not_elevated()` fails the test if it has.

## What refuses what, and what does not

`crates/kartei-testing/tests/birth_conditions.rs` refuses four things: a suite
running elevated, a temporary directory that outlives the test that owns it, two
temporary directories sharing a path, and `kartei-testing` appearing as a normal
dependency of anything, which is how a test helper ends up inside the shipped
binary.

Nothing refuses the rest. A test that opens a display, reaches the network,
writes outside its own directory, binds a fixed port directly instead of through
the helper, creates a symlink or installs a service passes every check in this
repository today. Those rules are read by a person in review and nowhere else,
and that is the whole of what stands behind them.

The GPU suite is kept out of the gate by construction rather than by
convention: the target is behind the `gpu` feature, which
`cargo test --workspace` does not enable. The cost is that the gate does not
compile it either, so it can rot without anything reporting it.
