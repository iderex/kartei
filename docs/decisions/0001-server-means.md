# 0001: The server is Rust

Status: accepted, in #3.

## Decision

The server is written in Rust. The replication engine runs inside the server
process and is called natively, with no foreign function boundary between the
engine and the code that applies updates.

This record is the means check for the server. It is written before the first
file that depends on the answer, and it is kept, because the code will later
show what was chosen and nothing will show what was rejected or why.

## What the engine choice forces

The three replication engines mature enough to build a product on are Rust
libraries. Measured against the repositories on 2026-08-06:

```
gh api repos/loro-dev/loro --jq '{lang:.language, pushed:.pushed_at}'
{"lang":"Rust","pushed":"2026-08-01T03:00:56Z"}
gh api repos/y-crdt/y-crdt --jq '{lang:.language, pushed:.pushed_at}'
{"lang":"Rust","pushed":"2026-08-05T10:51:45Z"}
gh api repos/automerge/automerge --jq '{lang:.language, pushed:.pushed_at}'
{"lang":"JavaScript","pushed":"2026-08-06T07:45:26Z"}
```

The third line is not the one #3 quotes, and the difference is the field rather
than the library. `.language` reports the repository's largest language by
bytes, and that repository holds the JavaScript and TypeScript packages
alongside the core:

```
gh api repos/automerge/automerge/languages --jq 'to_entries | sort_by(-.value) | .[:5] | map({(.key): .value}) | add'
{"C":410109,"CMake":34090,"JavaScript":4858093,"Rust":3666156,"TypeScript":509973}
```

So the core is Rust and the summary field says otherwise, which is exactly the
sort of cell that has to be read from the thing itself rather than from the
first field that answers quickly. The conclusion the sentence rests on holds:
all three cores are Rust.

Which engine is chosen is #11 and #12 and is not decided here. The point that
matters for the means is that it does not depend on which one wins.

## Why the engine has to run in the server at all

The server does not relay bytes. It applies updates, because the relational
views in M5 are projections computed from the document, and a projection
computed on a client is a projection a client can lie about. The server is
therefore a replica, and the language it is written in has to be one the engine
can be called from.

## Why native rather than a binding

Only one of the three publishes a binding for a garbage collected server
language, and it has not moved in a long time:

```
gh api repos/automerge/automerge-go --jq '{pushed:.pushed_at, stars:.stargazers_count}'
{"pushed":"2024-10-30T18:03:37Z","stars":127}
```

That binding is a wrapper over the Rust core rather than a reimplementation, so
choosing it means the tree carries Rust anyway and adds a foreign function
boundary on top. The boundary sits under the most correctness critical code in
the product: every conformance test crosses it, every fuzz finding has to be
read through it, and a crash inside the engine arrives as a stack trace the
host language cannot symbolise. Calling the engine natively removes that seam
instead of relocating it.

## What the deployment target forces

M8 is one executable with the database inside it, one configuration file and
one data directory. That is #59 and #60, and the costs are written down in
[0008-single-binary.md](0008-single-binary.md). Rust produces a single
artefact with no runtime to install, which is the whole argument of that
milestone rather than a convenience.

## Whether the toolchain can carry a gate that refuses things

A rule is only a rule if something refuses the violation, so the means has to
be able to carry the refusals the plan asks for. Four of them already exist in
the tree and are the same four the gate reports under their own names, in
`.github/workflows/gate.yml`: a build with warnings denied, the suite, a
formatting check that reports rather than rewrites, and a lint with warnings
denied.

The legs the parity milestone adds on top are owned by #52, #53, #55, #56 and
#57 rather than listed here. One of them was worth checking rather than
assuming, because it is the one that would have been an obstacle: static
analysis for Rust. The query packs are in the CodeQL repository:

```
gh api repos/github/codeql/contents/rust --jq '.[].name' | head -3
BUILD.bazel
README.md
ast-generator
gh api repos/github/codeql/contents/rust/ql --jq '.[].name' | head -3
.generated.list
.gitattributes
consistency-queries
```

That is evidence the packs exist. Whether the check runs, and on what, is #53.

## What is rejected

Go. It buys a simpler build, a faster compile and a larger pool of people who
can read the result, and it pays for it with a permanent foreign function seam
through the apply path. The seam is the cost measured above, and it does not
get smaller with time.

TypeScript on Node. The engines are available through WebAssembly, so the
engine is not the obstacle. The obstacles are that a single self contained
executable is awkward to produce, that the transitive dependency surface under
a document store would be much larger than this project wants to audit, and
that the fuzzing and mutation testing legs the parity milestone asks for are
considerably weaker there.

A managed runtime such as .NET or the JVM. No first party engine binding
exists, so the cost is the Go cost plus writing and maintaining the binding.

## What is forced rather than chosen

The browser client is TypeScript, because it runs in a browser. That force is
real, it is named, and it is held to its smallest surface: it stops at the
client. No server logic moves into it for convenience, and `web/README.md`
records that the client is deliberately outside `cargo build --workspace`, so a
Rust build never depends on a JavaScript toolchain being present.

## What this record does not measure

Build time and contributor onboarding cost. Both are real costs of this
decision and neither has a number here, because there is not enough of a
workspace yet to measure either honestly. They are named because they are the
costs most likely to be the ones that hurt, not because they have been
dismissed.

The engine comparison. This record establishes that the language follows from
the engines being Rust cores, not which core is best. #11 measures that.

## What would reverse this

A maintained first party binding of the chosen engine to a language with a
materially smaller build and operations cost, that passes the conformance suite
#13 defines against the same trait the native engine passes it against.

Or a measurement showing that the Rust build time or the contributor onboarding
cost is doing more damage to this project than the foreign function seam would
have. That is the pair of numbers the section above says are missing, so the
reversal condition is a measurement that does not exist yet rather than a
threshold that has already been passed.

Reversal is a new decision record, not an edit to this one.
