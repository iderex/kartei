# Contributing

This document exists because the gate points at it. The DCO check tells a
contributor whose commit is unsigned to read this file and `DCO`, so both have
to be here and both have to be right.

## A change starts as an issue

Planning happens on the tracker before the code that depends on it exists. An
issue says what is wrong, what the evidence for that is, and what "done" means.
Where the evidence is a number, it carries the command that produced it, so a
reader can run it again and get the same answer or find out that they cannot.

A decision that shapes the architecture is written down as a record under
`docs/decisions/`, using `docs/decisions/TEMPLATE.md`, before the first file
that depends on the answer. The code will later show what was chosen and
nothing will show what was rejected or why.

## Build and test

Install rustup. Nothing else is installed by hand: `rust-toolchain.toml` pins
the compiler and the two components the gate uses, and rustup reads that file,
so the first cargo command in a fresh clone fetches the pinned toolchain and
every command after it uses the same one the gate uses.

```
git clone https://github.com/iderex/kartei
cd kartei
cargo build --workspace --locked --all-targets
cargo test --workspace --locked
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Those four commands are the four checks the gate runs, in
`.github/workflows/gate.yml`, and they are written here in the same shape so
that a green run here means the same thing as a green run there. The build and
the lint deny warnings in the gate through `RUSTFLAGS`, which is the one
difference; export `RUSTFLAGS=-D warnings` before the build if you want the
same treatment locally, and the lint denies them on its own command line either
way.

`--locked` on purpose. It builds the committed lockfile rather than resolving a
new one, so a dependency added without updating `Cargo.lock` fails here instead
of quietly resolving to something nobody reviewed.

The gate has a fifth check, `msrv`, which builds the workspace with the floor
stated as `rust-version` in `Cargo.toml`. Reproduce it with
`rustup toolchain install 1.85.0 --profile minimal` and
`cargo +1.85.0 build --workspace --locked --all-targets`. The version is read
out of the manifest by the gate rather than typed into it, so if the two
disagree the manifest is right.

There is no step in this sequence that this document does not state, and no
argument in it that is not written here. That is the condition #8 asks for, and
it was checked by running the whole block in a clone made for the purpose.

## Tests

`docs/testing.md` holds the conditions every test in this repository meets, the
helpers in `kartei-testing` that make them cheap to meet, and the honest account
of which of them anything actually refuses. It is not restated here. A second
copy of that list would drift from the first, and it has already drifted once:
#7 asks for five rules in its Done-when and lists six, which is the sentence
being wrong rather than a rule being dropped.

The GPU suite is behind the `gpu` feature, so `cargo test --workspace` does not
build it. Run it deliberately, with
`cargo test -p kartei-testing --features gpu --test gpu`, when you have touched
something that could affect it.

## Sign your work

Every commit carries a `Signed-off-by` trailer naming its author, which is that
author asserting the Developer Certificate of Origin in `DCO`.

```
git commit -s
```

The check in `.github/workflows/dco.yml` walks every non-merge commit in the
pull request and compares the trailer against the commit's own author name and
email, so a trailer naming somebody else does not pass. If you have already
committed without one:

```
git rebase --signoff <base>
```

That rewrites the commits it touches, so do it before anyone else builds on the
branch.

## Branches and pull requests

Branch off `main` and open a pull request. Direct pushes to `main` are refused.

One topic per commit and per pull request. A commit message says what changed
and what failure it prevents, and where it corrects something, what was wrong
and how that was found.

The pull request body carries the evidence. A claim about the tree is quoted
with the command that produced it, run at the commit being pushed rather than
in your working copy, and a claim that cannot be backed by a command is written
as a claim rather than as a result. Where a change adds something that is meant
to refuse a defect, the body shows it refusing that defect: the check with the
defect present is red, the same check without it is green, and both are pasted.

Where the change builds an artefact, the body says what the artefact is made of
and why that fits, in a sentence. Which language, format or tool a thing is
made of is decided per artefact and never carried over from the last one.

## What refuses what, and what does not

The gate runs on every pull request and reds on a failure, and today nothing
makes it a condition of the merge:

```
gh api repos/iderex/kartei/rulesets/20485428 \
  --jq '[.rules[] | select(.type=="required_status_checks") | .parameters.required_status_checks[].context]'
[]
```

So a red gate blocks nothing by itself. `docs/gate.md` says which checks this
repository intends to require and why, #6 owns the naming discipline and the
required list, and #57 owns the comparison of the two. Until that lands, the
gate is read by a person.

`crates/kartei-server/tests/workflow_references.rs` refuses a repository path
that a workflow points a contributor at and that does not resolve, which is the
failure this document was written for. It reads the annotation messages the
workflows print and nothing else, so a path named in a comment, in a step body
or in a job summary is outside it and can go stale with nothing reporting it.

Nothing in this repository reads the rest of this document. Whether an issue
states its evidence, whether a pull request body carries the commands behind its
claims, whether a commit message says what it prevents, and whether a decision
record was written before the code that depends on it are all read by a person
in review and nowhere else.
