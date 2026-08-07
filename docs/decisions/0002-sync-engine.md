# 0002: What the three candidate replication engines do, measured

Status: accepted, in #11.

## Decision

The engine is chosen from the table below, and the table is produced by a
program in this repository rather than from reputation, stars or a README. It
runs with one command:

```
cargo bench -p kartei-sync --features measure
```

Which engine is adopted is #12 and is not settled here. This record is the
evidence that decision has to answer to, and it is written first so that the
adoption is a reading of a measurement rather than the measurement being
reverse-engineered from a preference.

The means for the measurement is a Rust program in the repository's own
workspace, calling each library natively, because that is the position the
engine will actually be used from. [0001-server-means.md](0001-server-means.md)
established that the server is Rust and that the engine runs in-process, so a
comparison made through any other surface would be measuring something the
product does not do. It is a bench target with no libtest harness rather than a
test, because what it produces is numbers a person reads against this table and
not a verdict, and a green tick that says nothing about the numbers would be
worse than no tick.

## The table

Versions are pinned exactly in `crates/kartei-sync/Cargo.toml`: automerge
0.6.1, loro 1.13.9, yrs 0.23.5. Every cell marked `measured` is a line printed
by the command above. Every cell marked `read from the source` was read out of
the released crate rather than run, and says what was read. No cell in this
table came from a README.

| Property | automerge 0.6.1 | loro 1.13.9 | yrs 0.23.5 |
| --- | --- | --- | --- |
| Convergence under arbitrary interleaving | all 6 delivery orders agree (measured) | all 6 delivery orders agree (measured) | all 6 delivery orders agree (measured) |
| Concurrent insertion at one position | 2 runs, `">BBBBBAAAAA<"` (measured) | 2 runs, `">AAAAABBBBB<"` (measured) | 2 runs, `">AAAAABBBBB<"` (measured) |
| Concurrent formatting over overlapping ranges | overlap not merged into a run: reports `0..6 bold, 4..10 italic` as two marks with offsets (measured) | overlap becomes its own run: `"0123" bold, "45" bold+italic, "6789" italic` (measured) | same three runs as loro (measured) |
| Move as a primitive | none; delete plus insert leaves the item in two places, 5 items out of a 4 item list (measured) | movable list, one item out (measured) | `Array::move_to`, one item out (measured) |
| History size, 5000 committed operations, 4000 chars | 19719 bytes (measured) | 30130 bytes (measured) | 40204 bytes (measured) |
| Trimming history | none exposed (measured as an absence) | shallow snapshot, 30130 to 12202 bytes (measured) | none that drops a prefix (measured as an absence) |
| Cost of embedding, crates compiled | 47 (measured) | 114 (measured) | 34 (measured) |
| Format stability across a library upgrade | `StringMigration` in `LoadOptions` converts documents written before strings became text (read from the source) | `EncodedBlobMode` carries `OutdatedSnapshot` and `OutdatedRle` beside the current modes (read from the source) | two update encodings, `encode_state_as_update_v1` and `_v2`, both on the transaction (read from the source) |

## What forced it

The properties in the table are #11's, and they are the product's rather than
the field's. A board is a reorderable list and a page is a tree that gets
reparented, so a move that duplicates a card is a defect a user sees. A block
editor is text with marks over ranges, so what happens when two people format
overlapping ranges is a rendering question and not a theoretical one. A server
holding thousands of documents for years cannot keep every keystroke, which is
why trimming is in the table at all, and #75 makes it a data protection
mechanism as well as a storage one.

The pinned versions are exact rather than ranges. A comparison whose numbers
move when a patch release lands is a comparison nobody can reproduce, and the
table above quotes versions beside every cell for the same reason.

## How each measured cell was produced

The construction of each measurement is written beside the code in
`crates/kartei-sync/benches/engine_comparison.rs`, because a comparison is worth
what the fairness of its arms is worth and a reader has to be able to check
that. Four points about the construction matter enough to repeat here.

The move arms are deliberately not the same code. Where a library has a move
primitive the primitive is used; where it has none, delete plus insert is used,
because that is what an application built on it would have to write. The
inequality between the arms is the result rather than a flaw in the harness.

The history workload is 5000 operations, each its own commit, one character
inserted at a position that moves around the document, with every tenth
operation deleting instead. Appending the same character at the end 5000 times
is the best case for a run-length encoder rather than a document, and an earlier
version of this harness did exactly that and reported 207 bytes for automerge
against 425 for loro. The character count of the resulting document is printed
beside the byte counts, and all three report 4000, which is what makes the three
numbers comparable at all.

The convergence arm exhausts all six delivery orders rather than sampling, so
there is no seed and no argument about coverage. All three pass, so that row
proves nothing about the libraries; what it establishes is that the harness
which would catch a violation exists and has been run.

The crate counts come from three throwaway packages, one per engine, each
depending on nothing else, counted with

```
cargo tree -e normal,build --prefix none | sed 's/ (\*)$//;s/ (proc-macro)$//' \
  | grep -E '^[a-zA-Z0-9_+.-]+ v' | sort -u | wc -l
```

minus the probe package itself. They are the crates that compile beside each
engine, not the packages a resolver enumerates, which is a much larger and much
less meaningful number.

## What it buys

The row that separates the three is the move row, and it separates them by more
than a margin. Two concurrent moves of one item leave that item in two places
under delete plus insert, and the harness shows the four item list coming back
with five items and the moved card appearing twice. Both of the other two
libraries return four items. For a product whose main surface is a reorderable
board, that is close to decisive on its own, and it is now a measurement rather
than an argument.

The second row that separates them is trimming. One of the three can cut its
history and the other two cannot, and the cut is worth a factor of about 2.5 on
this workload.

## What it costs

Holding three engines side by side to produce this table costs the repository
its clean dependency policy while they are all present. `deny.toml` bans
duplicate versions outright, and the three engines together bring eleven crates
at more than one version, none of them inside a crate this repository writes.
The skip list added for them is in `deny.toml` with a reason on every entry and
its own removal condition: the entries whose sources are all under the two
engines that lose disappear when #12 lands, and the checker reports an entry
that no longer matches as unused, so the tree says which ones those are.

That cost is real and it is paid for the comparison rather than for the product.
The engines are optional dependencies behind the `measure` feature and the bench
target declares that feature as required, so the default build carries none of
them and no gate leg compiles them. The dependency policy judges the graph with
all features on, which is why it sees them anyway, and that is the policy working
rather than a hole in it.

## What is rejected

Nothing is rejected here. This record measures; #12 chooses. Writing a rejection
into this file would settle that issue in the wrong place, and the rejection
reasons belong beside the choice they justify.

## What this record does not measure

Performance. Nothing in the harness is timed. A timing produced on one machine
is the least transferable number on the page, and the decision this feeds is
about what the engines can express and what they cost to hold. Byte counts are
reported instead, because an exported document is the same size everywhere.

Multi-peer history size. The history row is one writer. A history written by
several peers carries their identifiers and their concurrency and is larger than
these numbers, and how much larger is not measured.

What a client holding a version below a trim point actually experiences. The
harness measures that the cut exists and what it saves. What happens to a
replica that is behind it is #17, and it is a behaviour to be defined rather
than a number to be read off.

Whether a newer release of any of these libraries reads a document written by
an older one. The format stability row reports that each library carries
machinery for exactly that case, which is evidence that the case is handled
deliberately, and it is not evidence that any particular upgrade is safe. That
would need two versions of one library and a document written by the older,
which is a test rather than a reading, and it is not in this harness.

Compile time. It is a real cost of the crate counts in the table and it has no
number here, because the only figure available is from one machine.

Whether a trait can be drawn over all three at once. Each arm reaches into its
library's own API, and the shapes differ: two libraries return text already cut
into runs and one returns a list of marks with offsets. What that costs a common
trait is #13's to find out.

## What would reverse this

A measurement showing one of the rows above is wrong, which is what the harness
is committed for. Re-run it and compare; the versions are pinned so a
disagreement is a real disagreement rather than a version skew.

A change in what the product needs. The table is the product's properties rather
than the field's, so a plan that no longer has a reorderable board as its main
surface would weigh the move row differently.

Reversal is a new record, not an edit to this one.
