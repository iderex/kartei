# web

The browser client.

It is a directory in the workspace layout and not a Cargo member: the client is
built with its own toolchain and is deliberately outside `cargo build
--workspace`, so a Rust build never depends on a JavaScript toolchain being
present.

Nothing is here yet. The client lands with the milestones that define the
editor surface and the views it draws. What this file records is that the
client has a place, and where that place is, so the first change that adds one
does not have to decide it.
