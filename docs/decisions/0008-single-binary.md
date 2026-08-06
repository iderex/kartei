# 0008: One binary

Status: accepted, in #60.

## Decision

The product ships as one executable containing the server, the embedded
database, the web assets and the migrations. One configuration file. One data
directory. A container image wraps the same binary and adds nothing to it.

The decision is already implied by the storage decision in #20, which puts the
database inside the process. It is written down anyway, with its costs, because
a cost that is never stated gets rediscovered as a complaint.

## What it buys

An operator installs one thing and backs up one directory. There is no service
list, no compose file to keep current, no version skew between components, and
no network between parts of the product that can fail.

Upgrades are replacing a file. Migrations run at startup, which is #23.

The class of failures where one component is at one version and another is at a
different one does not exist, because there is one component.

## What it costs

Five costs. Each says whether anything mitigates it and names the issue that
does, rather than being written in a way that reads as handled. Two of them
have no owner, and saying so is the point of writing the list.

**No horizontal scale.** One process, one machine, one database file. This is
the storage decision restated at the deployment level rather than a separate
limit.

Not mitigated, and not intended to be. Where the ceiling is stated as a number,
and the condition on which it is revisited, is #20.

**The process is a single failure domain.** A panic anywhere takes the whole
server down: every open document, every connected client, every background job.
An importer parsing a hostile export and the apply loop serving edits are the
same process.

Reduced, not mitigated, and the difference matters. #71 runs importer fixtures
offline and treats their input as untrusted, and #16 fuzzes the update decoder,
which is the other place bytes chosen by somebody else reach a parser. Both
lower how often a panic happens. Neither isolates anything, so when one does
happen it still takes the process. No issue owns process isolation for the
importers, and nothing in the plan currently makes this cost smaller than it is
written here.

**Background work shares the process.** Imports, search indexing and history
trimming compete with the apply loop, which is the path a person waits on while
typing. Without a budget, each of them runs as fast as the machine allows and
the apply loop is what gets slower.

Not mitigated, and this one has a gap. #48 owns the limits on the transport,
which is a different surface: it bounds what a connected client can make the
server hold, not what the server's own background work may consume. #17 owns
what trimming costs and #26 owns search, each for its own feature. Nothing owns
the budget that holds between them, and until something does, the mitigation
named in this paragraph does not exist. The numbers such a budget would need
also wait on entry 6 of #1, which is the smallest machine the first release has
to run on.

**The binary is large,** because the web assets and the database engine are
inside it. Embedding the assets and fetching nothing at runtime is #61, and it
is the cause of part of this size rather than a mitigation of it. The container
image, its SBOM and its signature are #63.

Not mitigated. It is a fair trade and it is worth saying out loud rather than
being a surprise on a slow connection.

**Certificates and reverse proxying are the operator's job.** The product does
not obtain certificates and does not become a proxy, because that is where
single-binary products acquire their next surprise.

Documented rather than mitigated. What an operator has to put in front of the
binary belongs in #76, and the fact that the product does not defend the
transport itself belongs in the threat model, #77.

## What would reverse this

An operator population that needs more than one machine for one workspace. That
is the storage ceiling in #20 being reached rather than this decision failing on
its own terms, so the revisit condition is #20's numeric one and not a separate
one here.

A panic rate that makes the single failure domain the product's main complaint.
That reverses the second cost above and not the decision: the answer would be
isolating the importers, which is a change inside one binary.
