# The gate

This document is the target. It says which checks this repository intends to
require before a change reaches `main`, and why each one is there. It decides
nothing about how any of them is written.

The target is measured against the merge gate of `iderex/jellyfin-plugin-sso`,
which is the standard #50 adopts, adapted to a server rather than a plugin. Most
of what follows is therefore a correspondence: a check kept, a check translated
because the language forced it, a check dropped with its reason, or a check added
that the source gate has no equivalent for.

Nothing here is evidence that a check exists. Where a check is in the tree today
this document says so and the commands below print the truth; where it is not,
the entry is a plan and the issue that owes it is named.

## How the source gate is read

The list this document is measured against is not pasted here, because a pasted
list is a copy that goes stale in silence and then gets read as the authority.
Print it:

```
gh api repos/iderex/jellyfin-plugin-sso/rulesets --jq '.[] | select(.target == "branch") | .id' \
  | xargs -I{} gh api repos/iderex/jellyfin-plugin-sso/rulesets/{} \
    --jq '[.rules[] | select(.type == "required_status_checks")
                    | .parameters.required_status_checks[].context]'
```

That is the whole reading. If the source gate gains a check, loses one or renames
one, the output of that command stops matching the entries below, and the repair
is an edit to this document with a reason, not a drift somebody notices later.

The same for this repository, so the distance between the target and today is
something a reader prints rather than takes on trust:

```
gh api repos/iderex/kartei/rulesets --jq '.[] | select(.target == "branch") | .id' \
  | xargs -I{} gh api repos/iderex/kartei/rulesets/{} \
    --jq '{merge: ([.rules[] | select(.type == "pull_request")
                             | .parameters.allowed_merge_methods] | add),
           required: [.rules[] | select(.type == "required_status_checks")
                               | .parameters.required_status_checks[].context]}'
```

On the day this document landed that command reported no required checks at all
and three permitted merge methods. Both are stated again at the end, and neither
is repaired here.

## The name is the interface

A ruleset can require a check only by its literal check-run name, so a name in
this document is a promise and not a label. Renaming a job stops the rule
matching and nothing announces it, which is the failure the comment at the top of
`.github/workflows/gate.yml` is about.

Two naming habits are followed, and the second is an exception with a reason.
Names this repository chooses are lowercase and say what they are, which is what
the five checks already in the tree do. Names inherited from the source gate keep
their spelling exactly, so the correspondence is readable without a table, and
the analysis check keeps the source's shape with only the language changed.

## Kept, translated where the language forced it

`build` is a workspace build with warnings treated as errors. Same check, same
purpose, different compiler. In the tree.

`image` replaces `Package (JPRM) / Build package`, because the deliverable here is
a container image and a binary rather than a plugin package. Owed by #63.

`sbom` replaces `Package (JPRM) / Generate SBOM`, unchanged in purpose, since the
reason for shipping a bill of materials does not depend on what is packaged.
Owed by #63.

`Analyze (rust)` replaces `CodeQL` and `Analyze (csharp)` together. Same tool,
different language, and the language pack is in CodeQL's own tree rather than
being aspirational:

```
gh api repos/github/codeql/contents/rust --jq 'length'
```

Owed by #53, which also owns the condition that it produce a result on a pull
request from a fork.

`DCO sign-off` is kept exactly. In the tree, in `.github/workflows/dco.yml`.

`Deterministic PR-hygiene checks` is kept, with this repository's conventions in
place of the source's. Not in the tree.

`Enforce greppable invariants` is kept, with different invariants: no write to a
derived table outside the projection module, no query assembled by string
concatenation, no name from the sync engine above the sync boundary, and no
outbound network call in the server outside the importer module. The third of
those is partly covered already, and by a different reading:
`crates/kartei-server/tests/layout_boundary.rs` refuses an engine appearing
beneath three named crates on the real dependency graph, which is not the same
thing as refusing the engine's name in source text and does not cover the same
set of crates. Whether the grep is still worth adding on top of the graph check
is a question this entry leaves to the issue that implements it. Not in the tree.

`Reject Trojan Source Unicode` is kept exactly. In the tree, in
`.github/workflows/unicode-guard.yml`.

`Audit workflows (zizmor)` is kept exactly, because workflow files are the most
privileged code in the repository. In the tree, in `.github/workflows/zizmor.yml`.

`prettier` is kept for the web assets. A formatter for the server language is
added beside it rather than in place of it, because one formatter cannot cover
both languages. Not in the tree, and it has nothing to format yet: `web/` holds
only the note recording where the client will live.

`dependency-review` is kept exactly. In the tree, in
`.github/workflows/dependency-review.yml`.

## Dropped

`ABI floor build` is dropped, because there is no host process whose interface
has to be matched, so the check has nothing to check. The risk underneath it is
real here too, a contributor using something newer than the stated floor, and it
is covered by `msrv` below instead.

## Added

`test` is a named check separate from `build`. On the source board the suite runs
inside the build job; here the sync and storage suites are the product's main
evidence, so a green build must not be readable as a green suite. In the tree.

`format` checks and does not rewrite, because a gate that reformats hides the
thing it reports. In the tree.

`lint` denies warnings. It is the compiled-language equivalent of the analyzer
set the source board enforces through its warnings-as-errors build. In the tree.

`msrv` builds the whole workspace with the floor the workspace manifest states,
so the floor is exercised rather than asserted. It is what carries the risk the
dropped `ABI floor build` carried. In the tree, and it reads the number out of
`Cargo.toml` rather than restating it.

`fuzz` runs the update decoder's targets from the committed corpus for a bounded
time and prints the time it ran, so a short run cannot be read as a thorough one.
The decoder parses attacker-chosen binary from any authenticated user and is the
highest-consequence parser in the product. The source board has fuzzing and does
not require it. Owed by #55.

`simulation` runs every seed in the committed multi-peer corpus and prints the
seeds. The source board has no replicated state and so has no equivalent; here
this is the evidence that the product's central claim holds. Owed by #55.

`mutation` runs mutation testing on the sync and storage crates only, because
those two are where a passing suite is most likely to be measuring nothing. The
source board runs mutation testing and does not require it. Owed by #56.

`coverage` holds a floor on the sync and storage crates only. A repository-wide
number rewards testing whatever is easiest, which is not where the risk is. Owed
by #56.

`migrations` applies the full migration chain to a committed fixture database at
every released schema version. The source board ships no database of its own;
here a bad migration destroys installations that cannot be rolled back. Owed by
#23.

`dependency-policy` covers licences as well as advisories. `dependency-review`
covers the advisory side; the licence side is a compiled-tree concern, and it has
a side to enforce now that entry 1 of #1 is answered and this repository is
AGPL-3.0. Owed by #54.

`reproducible-build` rebuilds the release and compares the bytes. An operator
asked to run a binary on their own hardware should be able to produce it
themselves. This is a deliberate raise above the source gate rather than parity
with it. Owed by #58.

## Two things kept the same, stated rather than inherited silently

The source ruleset has no bypass actors and requires zero approving reviews.
Parity means the same here: no bypass, and no self-approval gesture. The residual
risk of a zero-approval gate on a solo project is real, is not closed by this
milestone, and is written down so it is not later mistaken for an oversight.

Merges are merge commits only, matching the source gate's allowed method.

## What this document does not do

It does not require anything. A ruleset requires a check; a document explains
which ones should be required and why. The two lists are compared by command
rather than by reading, and that comparison, along with the demonstration that
each check bites for the reason it names, is #57.

It does not create the issues it turns out to need. Writing the list out found
three target entries that no issue below the milestone parent owes:
`Deterministic PR-hygiene checks`, `Enforce greppable invariants` and
`prettier`. The search that found the gap is

```
gh api "search/issues?q=repo:iderex/kartei+is:issue+prettier+in:title,body" \
  --jq '[.items[].number]'
```

and it returns only #50 and #51, which are this milestone and this document.
Closing that gap is #50's to plan.

It does not claim the two lists match today. They do not. On the day this landed
the command in the second block above reported no required checks and permitted
merge, squash and rebase, against a target of a required list and merge commits
only. Both gaps belong to #57 and to the ruleset change it carries, and naming
them here is the disclosure, not the repair.

It also does not derive its own list from the tree, which every other list in
this repository is expected to do. It cannot: most of these checks do not exist
yet, so there is nothing to derive from, and a target that only named what
already exists would be an inventory. The commands above are what keep the drift
readable in the meantime, and the day the two lists do match, #57 is what says so
with a command rather than this document with a sentence.
