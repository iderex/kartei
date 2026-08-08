# Architecture

This note explains why the parts are arranged the way they are. It does not say
what they are, because the tree already answers that and answers it currently:
the workspace members are in [Cargo.toml](../Cargo.toml), and what each member
is for is the first doc comment in that crate's own library root. A list here
would drift against both within a month and would then be read as the authority
it is not.

Most of what follows is a design rather than something you can run. Where a
sentence is about code that is in the tree, it names the file. Where it is about
a design held on the tracker, it says so and names the issue. That difference is
load bearing: this note is a place to understand the product from, not evidence
that the product exists.

## The shape

One process, one data directory, one document model read two ways.

The server owns its own process and holds the database inside it, so there is no
component to be at a different version and no network between parts of the
product that can fail. A workspace is a set of replicated documents. A document
is read as a page of blocks and as rows in a table, and those are two readings
of one thing rather than two stores kept in step, which is why a card moved on a
board and a paragraph typed on a page are the same kind of event underneath.
Nothing leaves the host unless an operator configures something that sends it.

## Why anything sits behind a boundary

The replication engine is the highest consequence choice in the product and the
one with the least evidence behind it, because the field is young. The layout
assumes that choice may turn out to be wrong and keeps the cost of being wrong
inside one crate: only [crates/kartei-sync/src/lib.rs](../crates/kartei-sync/src/lib.rs)
is allowed to name an engine, and it may name one only behind a feature that is
off by default. Everything above it talks about documents, updates and versions,
which are that crate's own types.

A doc comment saying so would refuse nothing, and one line added to one
`[dependencies]` table would end it with nothing announcing the change. So the
boundary is read off the real dependency graph and the crossings are refused, in
[crates/kartei-server/tests/layout_boundary.rs](../crates/kartei-server/tests/layout_boundary.rs).
That test is the authority for which crate may know about which. This paragraph
is an explanation of it.

The engine-free half of that test asserts nothing today and prints that it did,
because no engine is chosen and no crate depends on one. It starts biting when
one is.

Why the server is written in the language it is, why the engine is called
natively rather than through a binding, and what was rejected, are in
[0001-server-means.md](decisions/0001-server-means.md). Which engine is chosen
is #11 and #12, and they have one record between them.
[0002-sync-engine.md](decisions/0002-sync-engine.md) is #11's: it measures the
candidates against what this product needs and deliberately picks none of them,
so that the pick is a reading of a measurement rather than the measurement being
written backwards from a preference. The pick is #12 and has no record yet, so
that half of the reasoning is on the tracker and not in this tree.

The browser client is the one place where the means is forced rather than
chosen, and the force is held to the client and stops there. It is outside
`cargo build --workspace` on purpose, so a Rust build never needs a JavaScript
toolchain present. [web/README.md](../web/README.md) is where that is recorded.

## The path an edit takes

None of this path is built. It is the design the milestones are cut along, and a
new contributor needs it because no single file shows it.

A keystroke becomes an editor transaction in the browser. The binding turns that
transaction into operations on the local replica of the document, so the letter
appears without waiting for anything, and the same operations go to the server
as an update. The binding is written in this repository rather than depended on,
because a defect there is indistinguishable from a defect in the engine while
being nothing of the kind, and that is #30.

The server is a replica rather than a relay. It applies the update itself,
because the table view is a projection computed from the document, and a
projection computed on a client is one a client can lie about. So the update
arrives, the server checks it is causally ready, applies it, runs the repair
pass that #18 defines over states two legal concurrent edits can produce, and
writes the update, the new version and the changed projection rows. All of that
sits in one transaction, which is #22: a crash between any two of those steps
would otherwise leave a table showing rows the document does not contain.

The update then goes out to the other replicas connected to that document, and a
replica that was away comes back and asks for what it missed rather than for
everything, which is #47. What a client is allowed to receive is decided at the
socket rather than in the document, because a replicated document has no concept
of a refused operation. That is #45, and it is the reason the permission model
sits in the transport milestone at all.

## What an import says it did

An importer reads somebody's existing workspace, and the part of that job that
goes wrong quietly is the part it could not carry across. So an import produces
an account of itself rather than relying on a fidelity note in a document, which
would describe the importer as it was when somebody last looked and would then be
trusted anyway. The shape of that account is
[crates/kartei-import/src/report.rs](../crates/kartei-import/src/report.rs), and
what each state means and why they may not be collapsed is argued there rather
than here.

One thing about it belongs in this note because it is a rule about the whole
product and not about one crate: an import that skipped nothing says so in
words. A report that is empty because everything came across and a report that is
empty because the importer stopped looking are otherwise the same bytes, and the
first is worth nothing if the reader cannot tell it from the second. The same
reasoning is why the `test` job in the gate prints what it covered.

## The ceilings

One process, one machine, one data directory. There is no horizontal scale and
none is intended, and the costs of that are written out one by one in
[0008-single-binary.md](decisions/0008-single-binary.md), including the two that
nothing in the plan currently makes smaller.

The storage ceiling is the number that decides when the first of those costs
starts to hurt. It belongs to #20, which has no record in `docs/decisions/` yet,
so this note has nothing to point at for it and does not invent one.

The gate that has to pass before any of this lands is
[.github/workflows/gate.yml](../.github/workflows/gate.yml), with the compiler
it runs pinned in [rust-toolchain.toml](../rust-toolchain.toml). What a test in
this repository may assume about the machine it runs on, and which of those
rules something actually refuses, is [testing.md](testing.md).

## The decision records

They live in `docs/decisions/`, numbered, one decision per file, and they are
added rather than rewritten. A record says what was decided, what it costs, what
it does not measure, and what would reverse it. Reversal is a new record that
names the old one, not an edit to the old one, because the code will later show
what was chosen and nothing will show what was rejected or why.

A new record is owed when a choice would be expensive to reverse and its reasons
would otherwise survive only in somebody's memory. Choosing what a thing is made
of is always such a choice: [0001-server-means.md](decisions/0001-server-means.md)
is the means check for the server and was written before the first file that
depended on the answer.

The numbering is deliberately not contiguous. A record is numbered after the
issue that settled it, so a gap is a decision that has not been made rather than
a record that went missing.

## What this note is checked for

Every path it names has to resolve in the tree, which is
[crates/kartei-server/tests/architecture_paths.rs](../crates/kartei-server/tests/architecture_paths.rs).
A rename that leaves this document pointing at nothing reds the suite rather
than being found by a reader following a dead link. What that check does not
reach is written in its own doc comment rather than here.

Nothing reads a sentence in this note about what the tree does not hold. Two are
here: that the pick between the engines has no record yet, and that the storage
ceiling has none. Neither names a path, so there is nothing for a path check to
resolve, and what each asserts is an absence, which is the one thing a check
built on resolving paths cannot see. Both stop being true when the record
arrives rather than when anything changes here, and one of them already had,
which is #104. They are read by a person and by nothing else.
