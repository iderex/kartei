// kartei, a self-hosted workspace for documents and structured data.
// Copyright (C) 2026  iderex
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! The three candidate engines, measured against the properties this product
//! needs, on one run.
//!
//! Run it:
//!
//! ```text
//! cargo bench -p kartei-sync --features measure
//! ```
//!
//! Every number in `docs/decisions/0002-sync-engine.md` that is marked as
//! measured comes out of this program. A cell in that table that this program
//! does not produce is marked as cited or as not measured, and the difference
//! between those three words is the whole point of the table.
//!
//! ## What this is not
//!
//! It is not a performance benchmark. Nothing here is timed, because the
//! decision this feeds is about what the engines can express and what they
//! cost to hold, and a timing on one laptop would be the least transferable
//! number on the page. Sizes in bytes are reported instead, because a byte
//! count of an exported document is the same on every machine.
//!
//! It is not a conformance suite either. That is #13, it is written against the
//! trait rather than against three libraries, and it judges one engine at a
//! time. This program deliberately reaches into each library's own API, because
//! the question it answers is which of them a trait could be drawn over at all.
//!
//! ## Why each measurement is shaped the way it is
//!
//! Each section states its own construction beside the code, because a
//! comparison is only as good as the fairness of the thing being compared, and
//! the reader has to be able to check that the three arms are doing the same
//! work.

use std::fmt::Write as _;

/// Every measurement prints under a heading, so a run can be read top to bottom
/// against the table in the record.
fn heading(title: &str) {
    println!();
    println!("== {title}");
}

fn line(engine: &str, result: &str) {
    println!("   {engine:<10} {result}");
}

fn main() {
    println!("Engine comparison. Versions are pinned exactly in Cargo.toml:");
    println!("   automerge  0.6.1");
    println!("   loro       1.13.9");
    println!("   yrs        0.23.5");

    convergence_under_interleaving();
    concurrent_insertion_at_one_position();
    concurrent_overlapping_formatting();
    concurrent_move_of_one_item();
    history_size_and_trimming();

    println!();
    println!("Done. Every line above is a measurement; nothing here is a claim.");
}

// ---------------------------------------------------------------------------
// Convergence under arbitrary interleaving
// ---------------------------------------------------------------------------

/// Three replicas each make local edits without seeing each other, and their
/// updates are then delivered to a fresh fourth replica in every one of the six
/// possible orders.
///
/// The measurement is whether all six orders reach the same text. All three
/// libraries claim this property, so a pass proves nothing new; the value is
/// that the harness which would catch a violation exists and has been run,
/// which is the difference between the claim and the evidence.
///
/// Six orders rather than a random sample, because with three update blobs the
/// whole permutation set is small enough to exhaust, and an exhausted space
/// needs no seed and no argument about coverage.
fn convergence_under_interleaving() {
    heading("Convergence under arbitrary interleaving (6 delivery orders, 3 replicas)");

    line("automerge", &automerge_convergence());
    line("loro", &loro_convergence());
    line("yrs", &yrs_convergence());
}

/// The six permutations of three items, written out rather than generated, so
/// the set being tested is visible at the call site.
const ORDERS: [[usize; 3]; 6] = [
    [0, 1, 2],
    [0, 2, 1],
    [1, 0, 2],
    [1, 2, 0],
    [2, 0, 1],
    [2, 1, 0],
];

/// The same verdict sentence for all three arms, so the three lines in the
/// output are comparable at a glance.
fn convergence_verdict(results: &[String]) -> String {
    let first = &results[0];
    if results.iter().all(|r| r == first) {
        format!(
            "all 6 orders agree, final length {} chars",
            first.chars().count()
        )
    } else {
        let mut distinct: Vec<&String> = Vec::new();
        for r in results {
            if !distinct.contains(&r) {
                distinct.push(r);
            }
        }
        format!("DIVERGED: {} distinct results out of 6", distinct.len())
    }
}

fn automerge_convergence() -> String {
    use automerge::{AutoCommit, ObjType, ROOT, ReadDoc, transaction::Transactable};

    // One common ancestor, so the three replicas are concurrent with each other
    // and not with the creation of the text container itself.
    let mut base = AutoCommit::new();
    let text = base
        .put_object(ROOT, "body", ObjType::Text)
        .expect("put text container");
    base.splice_text(&text, 0, 0, "start")
        .expect("seed the text");
    let seed = base.save();

    let mut updates = Vec::new();
    for (i, word) in ["-alpha", "-beta", "-gamma"].iter().enumerate() {
        let mut replica = AutoCommit::load(&seed).expect("load the seed");
        replica.set_actor(automerge::ActorId::from(vec![i as u8 + 1]));
        // Each replica appends at its own end position, which is the same index
        // on every replica because none of them has seen the others.
        let len = replica.length(&text);
        replica
            .splice_text(&text, len, 0, word)
            .expect("append the word");
        updates.push(replica);
    }

    let mut results = Vec::new();
    for order in ORDERS {
        let mut doc = AutoCommit::load(&seed).expect("load the seed");
        for i in order {
            let mut other = updates[i].fork();
            doc.merge(&mut other).expect("merge a replica");
        }
        results.push(doc.text(&text).expect("read the text"));
    }
    convergence_verdict(&results)
}

fn loro_convergence() -> String {
    use loro::{ExportMode, LoroDoc};

    let base = LoroDoc::new();
    base.set_peer_id(9).expect("set the peer id");
    base.get_text("body").insert(0, "start").expect("seed");
    base.commit();
    let seed = base
        .export(ExportMode::Snapshot)
        .expect("export the seed snapshot");

    let mut updates = Vec::new();
    for (i, word) in ["-alpha", "-beta", "-gamma"].iter().enumerate() {
        let replica = LoroDoc::new();
        replica.import(&seed).expect("import the seed");
        replica.set_peer_id(i as u64 + 1).expect("set the peer id");
        let before = replica.oplog_vv();
        let text = replica.get_text("body");
        let len = text.len_unicode();
        text.insert(len, word).expect("append the word");
        replica.commit();
        updates.push(
            replica
                .export(ExportMode::updates(&before))
                .expect("export the local updates"),
        );
    }
    // Dropped so the loop below cannot accidentally read a live replica instead
    // of the bytes that replica produced.
    drop(base);

    let mut results = Vec::new();
    for order in ORDERS {
        let doc = LoroDoc::new();
        doc.import(&seed).expect("import the seed");
        for i in order {
            doc.import(&updates[i]).expect("import a replica's updates");
        }
        results.push(doc.get_text("body").to_string());
    }
    convergence_verdict(&results)
}

fn yrs_convergence() -> String {
    use yrs::{
        Doc, GetString, ReadTxn, StateVector, Text, Transact, Update, updates::decoder::Decode as _,
    };

    let base = Doc::with_client_id(9);
    {
        let text = base.get_or_insert_text("body");
        let mut txn = base.transact_mut();
        text.insert(&mut txn, 0, "start");
    }
    let seed = base
        .transact()
        .encode_state_as_update_v1(&StateVector::default());

    let mut updates = Vec::new();
    for (i, word) in ["-alpha", "-beta", "-gamma"].iter().enumerate() {
        let replica = Doc::with_client_id(i as u64 + 1);
        {
            let mut txn = replica.transact_mut();
            txn.apply_update(Update::decode_v1(&seed).expect("decode the seed"))
                .expect("apply the seed");
        }
        let before = replica.transact().state_vector();
        {
            let text = replica.get_or_insert_text("body");
            let mut txn = replica.transact_mut();
            let len = text.get_string(&txn).chars().count() as u32;
            text.insert(&mut txn, len, word);
        }
        updates.push(replica.transact().encode_state_as_update_v1(&before));
    }

    let mut results = Vec::new();
    for order in ORDERS {
        let doc = Doc::with_client_id(100);
        {
            let mut txn = doc.transact_mut();
            txn.apply_update(Update::decode_v1(&seed).expect("decode the seed"))
                .expect("apply the seed");
            for i in order {
                txn.apply_update(Update::decode_v1(&updates[i]).expect("decode an update"))
                    .expect("apply an update");
            }
        }
        let text = doc.get_or_insert_text("body");
        let txn = doc.transact();
        results.push(text.get_string(&txn));
    }
    convergence_verdict(&results)
}

// ---------------------------------------------------------------------------
// Concurrent insertion at one position
// ---------------------------------------------------------------------------

/// Two replicas type a run of characters at the same offset, without seeing
/// each other, and the two runs are then merged.
///
/// What is being measured is whether the result holds the two runs whole or
/// shreds them into each other. Both outcomes converge, so a convergence test
/// cannot tell them apart, and the difference is what a person sees on the
/// screen: `AAAAABBBBB` is two people typing, `ABABABABAB` is neither of them.
///
/// The count reported is the number of maximal runs of one character, taken
/// over the inserted region only. The seed text is two sentinel characters
/// around the insertion point, and counting those would add two runs to every
/// engine's answer and make the good result read as `4`, so they are cut off
/// before the count rather than subtracted from it afterwards.
fn concurrent_insertion_at_one_position() {
    heading("Concurrent insertion at one position (two runs of 5, merged)");
    println!("   Reported as: the merged string, and how many maximal single-character runs");
    println!("   the region between the two sentinels holds. 2 means each typist's run");
    println!("   survived whole; 10 means they interleaved letter by letter.");

    line("automerge", &automerge_same_position());
    line("loro", &loro_same_position());
    line("yrs", &yrs_same_position());
}

/// Counts maximal runs of one repeated character.
fn runs_of(s: &str) -> usize {
    let mut runs = 0;
    let mut previous = None;
    for c in s.chars() {
        if Some(c) != previous {
            runs += 1;
            previous = Some(c);
        }
    }
    runs
}

/// The region the two typists inserted into, which is everything strictly
/// between the sentinels. A merged string that has lost a sentinel is reported
/// as such rather than being counted, because the count would then be over a
/// region this function guessed at.
fn between_sentinels(merged: &str) -> Option<&str> {
    let open = merged.find('>')?;
    let close = merged.rfind('<')?;
    if close <= open {
        return None;
    }
    Some(&merged[open + 1..close])
}

fn describe_merge(merged: &str) -> String {
    match between_sentinels(merged) {
        Some(region) => format!(
            "{:?}, {} runs between the sentinels",
            merged,
            runs_of(region)
        ),
        None => format!("{merged:?}, sentinels missing, not counted"),
    }
}

fn automerge_same_position() -> String {
    use automerge::{AutoCommit, ObjType, ROOT, transaction::Transactable};

    let mut base = AutoCommit::new();
    let text = base.put_object(ROOT, "body", ObjType::Text).expect("put");
    base.splice_text(&text, 0, 0, "><").expect("seed");
    let seed = base.save();

    let mut left = AutoCommit::load(&seed).expect("load");
    left.set_actor(automerge::ActorId::from(vec![1]));
    left.splice_text(&text, 1, 0, "AAAAA").expect("left types");

    let mut right = AutoCommit::load(&seed).expect("load");
    right.set_actor(automerge::ActorId::from(vec![2]));
    right
        .splice_text(&text, 1, 0, "BBBBB")
        .expect("right types");

    left.merge(&mut right).expect("merge");
    let merged = {
        use automerge::ReadDoc as _;
        left.text(&text).expect("read")
    };
    describe_merge(&merged)
}

fn loro_same_position() -> String {
    use loro::{ExportMode, LoroDoc};

    let base = LoroDoc::new();
    base.set_peer_id(9).expect("peer");
    base.get_text("body").insert(0, "><").expect("seed");
    base.commit();
    let seed = base.export(ExportMode::Snapshot).expect("export");

    let left = LoroDoc::new();
    left.import(&seed).expect("import");
    left.set_peer_id(1).expect("peer");
    left.get_text("body").insert(1, "AAAAA").expect("left");
    left.commit();

    let right = LoroDoc::new();
    right.import(&seed).expect("import");
    right.set_peer_id(2).expect("peer");
    right.get_text("body").insert(1, "BBBBB").expect("right");
    right.commit();

    let left_bytes = left.export(ExportMode::Snapshot).expect("export");
    right.import(&left_bytes).expect("merge");
    describe_merge(&right.get_text("body").to_string())
}

fn yrs_same_position() -> String {
    use yrs::{
        Doc, GetString, ReadTxn, StateVector, Text, Transact, Update, updates::decoder::Decode as _,
    };

    let base = Doc::with_client_id(9);
    {
        let text = base.get_or_insert_text("body");
        let mut txn = base.transact_mut();
        text.insert(&mut txn, 0, "><");
    }
    let seed = base
        .transact()
        .encode_state_as_update_v1(&StateVector::default());

    let make = |client: u64, run: &str| -> Doc {
        let doc = Doc::with_client_id(client);
        {
            let mut txn = doc.transact_mut();
            txn.apply_update(Update::decode_v1(&seed).expect("decode"))
                .expect("apply");
        }
        {
            let text = doc.get_or_insert_text("body");
            let mut txn = doc.transact_mut();
            text.insert(&mut txn, 1, run);
        }
        doc
    };

    let left = make(1, "AAAAA");
    let right = make(2, "BBBBB");

    let from_left = left
        .transact()
        .encode_state_as_update_v1(&right.transact().state_vector());
    {
        let mut txn = right.transact_mut();
        txn.apply_update(Update::decode_v1(&from_left).expect("decode"))
            .expect("apply");
    }
    let text = right.get_or_insert_text("body");
    let txn = right.transact();
    describe_merge(&text.get_string(&txn))
}

// ---------------------------------------------------------------------------
// Concurrent formatting over overlapping ranges
// ---------------------------------------------------------------------------

/// Two replicas apply two different marks over two ranges that overlap in the
/// middle, without seeing each other, and the two are then merged.
///
/// The text is `0123456789`. One replica marks `0..6` bold, the other marks
/// `4..10` italic, so `4..6` is claimed by both. What is being measured is
/// whether the merged document describes that overlap as a region carrying both
/// marks, which is a thing an editor can render and a person can understand, or
/// as something else.
///
/// Each engine's own representation is printed rather than a normalised one.
/// Normalising would hide the difference that matters here: automerge reports a
/// list of marks with offsets, while the other two report the text already cut
/// into runs. Both answer the question, and how each one answers it is part of
/// what a binding above the trait would have to carry.
fn concurrent_overlapping_formatting() {
    heading("Concurrent formatting over overlapping ranges (bold 0..6, italic 4..10)");
    println!("   The overlap is 4..6. Each engine reports in its own shape, which is the");
    println!("   second half of the measurement.");

    line("automerge", &automerge_formatting());
    line("loro", &loro_formatting());
    line("yrs", &yrs_formatting());
}

fn automerge_formatting() -> String {
    use automerge::{
        AutoCommit, ObjType, ROOT, ReadDoc, ScalarValue,
        marks::{ExpandMark, Mark},
        transaction::Transactable,
    };

    let mut base = AutoCommit::new();
    let text = base.put_object(ROOT, "body", ObjType::Text).expect("put");
    base.splice_text(&text, 0, 0, "0123456789").expect("seed");
    let seed = base.save();

    let mut left = AutoCommit::load(&seed).expect("load");
    left.set_actor(automerge::ActorId::from(vec![1]));
    left.mark(
        &text,
        Mark::new("bold".to_owned(), ScalarValue::Boolean(true), 0, 6),
        ExpandMark::None,
    )
    .expect("bold");

    let mut right = AutoCommit::load(&seed).expect("load");
    right.set_actor(automerge::ActorId::from(vec![2]));
    right
        .mark(
            &text,
            Mark::new("italic".to_owned(), ScalarValue::Boolean(true), 4, 10),
            ExpandMark::None,
        )
        .expect("italic");

    left.merge(&mut right).expect("merge");

    let marks = left.marks(&text).expect("read the marks");
    let rendered: Vec<String> = marks
        .iter()
        .map(|m| format!("{}..{} {}", m.start, m.end, m.name()))
        .collect();
    format!(
        "text {:?}, marks [{}]",
        left.text(&text).expect("read"),
        rendered.join(", ")
    )
}

fn loro_formatting() -> String {
    use loro::{ExportMode, LoroDoc, TextDelta};

    let base = LoroDoc::new();
    base.set_peer_id(9).expect("peer");
    base.get_text("body").insert(0, "0123456789").expect("seed");
    base.commit();
    let seed = base.export(ExportMode::Snapshot).expect("export");

    let left = LoroDoc::new();
    left.import(&seed).expect("import");
    left.set_peer_id(1).expect("peer");
    left.get_text("body")
        .mark(0..6, "bold", true)
        .expect("bold");
    left.commit();

    let right = LoroDoc::new();
    right.import(&seed).expect("import");
    right.set_peer_id(2).expect("peer");
    right
        .get_text("body")
        .mark(4..10, "italic", true)
        .expect("italic");
    right.commit();

    let left_bytes = left.export(ExportMode::Snapshot).expect("export");
    right.import(&left_bytes).expect("merge");

    let rendered: Vec<String> = right
        .get_text("body")
        .to_delta()
        .into_iter()
        .map(|d| match d {
            TextDelta::Insert { insert, attributes } => {
                let mut keys: Vec<String> = attributes
                    .map(|a| a.keys().cloned().collect())
                    .unwrap_or_default();
                keys.sort();
                if keys.is_empty() {
                    format!("{insert:?} plain")
                } else {
                    format!("{insert:?} {}", keys.join("+"))
                }
            }
            other => format!("{other:?}"),
        })
        .collect();
    format!("runs [{}]", rendered.join(", "))
}

fn yrs_formatting() -> String {
    use yrs::{
        Any, Doc, ReadTxn, StateVector, Text, Transact, Update, types::text::YChange,
        updates::decoder::Decode as _,
    };

    let base = Doc::with_client_id(9);
    {
        let text = base.get_or_insert_text("body");
        let mut txn = base.transact_mut();
        text.insert(&mut txn, 0, "0123456789");
    }
    let seed = base
        .transact()
        .encode_state_as_update_v1(&StateVector::default());

    let make = |client: u64, index: u32, len: u32, key: &str| -> Doc {
        let doc = Doc::with_client_id(client);
        {
            let mut txn = doc.transact_mut();
            txn.apply_update(Update::decode_v1(&seed).expect("decode"))
                .expect("apply");
        }
        {
            let text = doc.get_or_insert_text("body");
            let mut txn = doc.transact_mut();
            let attrs = yrs::types::Attrs::from([(key.into(), Any::Bool(true))]);
            text.format(&mut txn, index, len, attrs);
        }
        doc
    };

    let left = make(1, 0, 6, "bold");
    let right = make(2, 4, 6, "italic");

    let from_left = left
        .transact()
        .encode_state_as_update_v1(&right.transact().state_vector());
    {
        let mut txn = right.transact_mut();
        txn.apply_update(Update::decode_v1(&from_left).expect("decode"))
            .expect("apply");
    }

    let text = right.get_or_insert_text("body");
    let txn = right.transact();
    let rendered: Vec<String> = text
        .diff(&txn, YChange::identity)
        .into_iter()
        .map(|chunk| {
            let mut keys: Vec<String> = chunk
                .attributes
                .map(|a| a.keys().map(|k| k.to_string()).collect())
                .unwrap_or_default();
            keys.sort();
            let content = match chunk.insert {
                yrs::Out::Any(Any::String(s)) => s.to_string(),
                other => format!("{other:?}"),
            };
            if keys.is_empty() {
                format!("{content:?} plain")
            } else {
                format!("{content:?} {}", keys.join("+"))
            }
        })
        .collect();
    format!("runs [{}]", rendered.join(", "))
}

// ---------------------------------------------------------------------------
// Move as a primitive
// ---------------------------------------------------------------------------

/// Two replicas move the same item to two different places, without seeing each
/// other, and the two moves are then merged.
///
/// This is the measurement that matters most for this product, because a board
/// is a reorderable list and reparenting a page is the same operation in a
/// tree. Modelled as delete plus insert, two concurrent moves of one item leave
/// that item in two places, and the reported count is what shows it: `a`
/// appearing twice in a four item list is the failure, and it is a failure a
/// user sees as a duplicated card.
///
/// Each arm uses the best move the library offers. Where the library has a move
/// primitive the primitive is used; where it does not, delete plus insert is
/// used, because that is what an application built on it would have to do.
/// The arms are therefore not the same code, and that inequality is the result
/// rather than a flaw in the harness.
fn concurrent_move_of_one_item() {
    heading("Concurrent move of one item (two replicas move `a`, then merge)");
    println!("   Start [a, b, c, d]. One replica moves `a` to the end, the other moves it");
    println!("   between `b` and `c`. Four items out means the item survived as one item.");

    line("automerge", &automerge_move());
    line("loro", &loro_move());
    line("yrs", &yrs_move());
}

fn describe_list(items: &[String], mechanism: &str) -> String {
    let mut rendered = String::new();
    rendered.push('[');
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            rendered.push_str(", ");
        }
        let _ = write!(rendered, "{item}");
    }
    rendered.push(']');
    let copies = items.iter().filter(|i| i.as_str() == "a").count();
    format!(
        "{mechanism}: {rendered}, {} items, `a` appears {copies} time(s)",
        items.len()
    )
}

fn automerge_move() -> String {
    use automerge::{AutoCommit, ObjType, ROOT, ReadDoc, transaction::Transactable};

    let mut base = AutoCommit::new();
    let list = base.put_object(ROOT, "cards", ObjType::List).expect("put");
    for (i, name) in ["a", "b", "c", "d"].iter().enumerate() {
        base.insert(&list, i, *name).expect("seed");
    }
    let seed = base.save();

    // No move operation exists in this library's transaction interface, so an
    // application has to delete and re-insert, which is the shape the issue
    // names. Both replicas do exactly that.
    let mut left = AutoCommit::load(&seed).expect("load");
    left.set_actor(automerge::ActorId::from(vec![1]));
    left.delete(&list, 0).expect("delete");
    left.insert(&list, 3, "a").expect("insert at the end");

    let mut right = AutoCommit::load(&seed).expect("load");
    right.set_actor(automerge::ActorId::from(vec![2]));
    right.delete(&list, 0).expect("delete");
    right.insert(&list, 1, "a").expect("insert in the middle");

    left.merge(&mut right).expect("merge");

    let mut items = Vec::new();
    for i in 0..left.length(&list) {
        let value = left.get(&list, i).expect("read").map(|(v, _)| v);
        items.push(match value {
            Some(v) => v.to_str().unwrap_or("?").to_owned(),
            None => "?".to_owned(),
        });
    }
    describe_list(&items, "delete plus insert (no move primitive)")
}

fn loro_move() -> String {
    use loro::{ExportMode, LoroDoc, LoroValue};

    let base = LoroDoc::new();
    base.set_peer_id(9).expect("peer");
    let seed_list = base.get_movable_list("cards");
    for (i, name) in ["a", "b", "c", "d"].iter().enumerate() {
        seed_list.insert(i, *name).expect("seed");
    }
    base.commit();
    let seed = base.export(ExportMode::Snapshot).expect("export");

    let left = LoroDoc::new();
    left.import(&seed).expect("import");
    left.set_peer_id(1).expect("peer");
    left.get_movable_list("cards").mov(0, 3).expect("move");
    left.commit();

    let right = LoroDoc::new();
    right.import(&seed).expect("import");
    right.set_peer_id(2).expect("peer");
    right.get_movable_list("cards").mov(0, 1).expect("move");
    right.commit();

    let left_bytes = left.export(ExportMode::Snapshot).expect("export");
    right.import(&left_bytes).expect("merge");

    let merged = right.get_movable_list("cards");
    let mut items = Vec::new();
    for i in 0..merged.len() {
        let rendered = match merged.get(i) {
            Some(loro::ValueOrContainer::Value(LoroValue::String(s))) => s.to_string(),
            other => format!("{other:?}"),
        };
        items.push(rendered);
    }
    describe_list(&items, "movable list, move primitive")
}

fn yrs_move() -> String {
    use yrs::{Array, Doc, ReadTxn, StateVector, Transact, Update, updates::decoder::Decode as _};

    let base = Doc::with_client_id(9);
    {
        let array = base.get_or_insert_array("cards");
        let mut txn = base.transact_mut();
        array.insert_range(&mut txn, 0, ["a", "b", "c", "d"]);
    }
    let seed = base
        .transact()
        .encode_state_as_update_v1(&StateVector::default());

    // This library does carry a move primitive on its array type, `move_to`,
    // which is the fact the harness exists to find rather than to assume.
    let make = |client: u64, target: u32| -> Doc {
        let doc = Doc::with_client_id(client);
        {
            let mut txn = doc.transact_mut();
            txn.apply_update(Update::decode_v1(&seed).expect("decode"))
                .expect("apply");
        }
        {
            let array = doc.get_or_insert_array("cards");
            let mut txn = doc.transact_mut();
            array.move_to(&mut txn, 0, target);
        }
        doc
    };

    let left = make(1, 4);
    let right = make(2, 2);

    let from_left = left
        .transact()
        .encode_state_as_update_v1(&right.transact().state_vector());
    {
        let mut txn = right.transact_mut();
        txn.apply_update(Update::decode_v1(&from_left).expect("decode"))
            .expect("apply");
    }

    let array = right.get_or_insert_array("cards");
    let txn = right.transact();
    let items: Vec<String> = array
        .iter(&txn)
        .map(|out| match out {
            yrs::Out::Any(yrs::Any::String(s)) => s.to_string(),
            other => format!("{other:?}"),
        })
        .collect();
    describe_list(&items, "array move primitive (`move_to`)")
}

// ---------------------------------------------------------------------------
// History size, and whether history can be trimmed
// ---------------------------------------------------------------------------

/// How many bytes a document costs to hold after a realistic amount of typing,
/// and whether the library offers a way to forget the beginning of its history.
///
/// The workload is the same for all three: `OPERATIONS` operations, each its
/// own commit, one character inserted at a position that moves around the
/// document, with every tenth operation deleting a character instead. Batching
/// the text into one operation would flatter every engine equally and measure
/// nothing about history, and appending the same character at the end every
/// time is the best case for a run-length encoder rather than a document.
///
/// The characters and the positions come from a small deterministic generator
/// written here rather than from a random number crate, so the workload is the
/// same on every machine and every run, and so this program adds no dependency
/// beyond the three being compared.
///
/// Three numbers per engine. The character count of the resulting document,
/// which is what makes the byte counts mean anything: two engines that
/// disagreed about how much text they hold would not be comparable, and the
/// count is printed rather than assumed. Then the whole document with its
/// history, which is what a server holding thousands of documents stores. Then
/// whatever the library offers as a way to keep the current state and drop the
/// history behind it, absent where there is none, and that absence is the
/// measurement.
///
/// One writer. A history written by several peers carries their identifiers
/// and their concurrency and is bigger than this, and how much bigger is not
/// measured here.
fn history_size_and_trimming() {
    const OPERATIONS: usize = 5_000;
    heading("History size after 5000 committed operations, and trimming");
    println!("   chars = the text the document holds at the end, so the byte counts are");
    println!("   comparable. full = the document with its history, as a server would store");
    println!("   it. trimmed = the same document after the library's own history cut.");

    line("automerge", &automerge_history(OPERATIONS));
    line("loro", &loro_history(OPERATIONS));
    line("yrs", &yrs_history(OPERATIONS));
}

/// The workload, as a list of edits, generated once and replayed identically
/// into each engine.
///
/// An `Edit` is an index and either a character to insert there or a deletion
/// of the character at that index. The indices are produced against a running
/// length so that every edit is in range for a document that has had all the
/// previous edits applied, which is what makes one list replayable into three
/// libraries whose position arithmetic is otherwise their own business.
enum Edit {
    Insert { at: usize, ch: char },
    Delete { at: usize },
}

/// A linear congruential generator with the constants from Numerical Recipes.
/// It is here because the workload has to be identical on every machine and a
/// generator is four lines, which is cheaper than a dependency this program
/// would then be the only reason for.
struct Lcg(u32);

impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
}

fn workload(operations: usize) -> Vec<Edit> {
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz ";
    let mut rng = Lcg(0x5eed_1234);
    let mut edits = Vec::with_capacity(operations);
    let mut length = 0usize;
    for i in 0..operations {
        // Every tenth operation deletes, so the log is not a pure insert run
        // and the engines are asked to carry tombstones as well as content.
        if i % 10 == 9 && length > 0 {
            let at = (rng.next() as usize) % length;
            edits.push(Edit::Delete { at });
            length -= 1;
        } else {
            let at = (rng.next() as usize) % (length + 1);
            let ch = ALPHABET[(rng.next() as usize) % ALPHABET.len()] as char;
            edits.push(Edit::Insert { at, ch });
            length += 1;
        }
    }
    edits
}

fn automerge_history(operations: usize) -> String {
    use automerge::{AutoCommit, ObjType, ROOT, ReadDoc, transaction::Transactable};

    let mut doc = AutoCommit::new();
    let text = doc.put_object(ROOT, "body", ObjType::Text).expect("put");
    for edit in workload(operations) {
        match edit {
            Edit::Insert { at, ch } => {
                let mut buffer = [0u8; 4];
                doc.splice_text(&text, at, 0, ch.encode_utf8(&mut buffer))
                    .expect("insert");
            }
            Edit::Delete { at } => {
                doc.splice_text(&text, at, 1, "").expect("delete");
            }
        }
        // A commit per operation, so the history holds one change per edit
        // rather than one change holding all of them.
        doc.commit();
    }
    let chars = doc.text(&text).expect("read").chars().count();
    let full = doc.save().len();
    format!(
        "{chars} chars, full {full} bytes, trimmed n/a: the library exposes no history cut, \
         so the whole log is what is stored"
    )
}

fn loro_history(operations: usize) -> String {
    use loro::{ExportMode, LoroDoc};

    let doc = LoroDoc::new();
    doc.set_peer_id(1).expect("peer");
    let text = doc.get_text("body");
    for edit in workload(operations) {
        match edit {
            Edit::Insert { at, ch } => {
                let mut buffer = [0u8; 4];
                text.insert(at, ch.encode_utf8(&mut buffer))
                    .expect("insert");
            }
            Edit::Delete { at } => text.delete(at, 1).expect("delete"),
        }
        doc.commit();
    }
    let chars = text.to_string().chars().count();
    let full = doc.export(ExportMode::Snapshot).expect("export").len();
    let frontiers = doc.oplog_frontiers();
    let trimmed = doc
        .export(ExportMode::shallow_snapshot(&frontiers))
        .expect("export shallow")
        .len();
    format!(
        "{chars} chars, full {full} bytes, trimmed {trimmed} bytes \
         via a shallow snapshot at the current frontiers"
    )
}

fn yrs_history(operations: usize) -> String {
    use yrs::{Doc, GetString, ReadTxn, StateVector, Text, Transact};

    let doc = Doc::with_client_id(1);
    let text = doc.get_or_insert_text("body");
    for edit in workload(operations) {
        // One transaction per operation, so the update log holds one item per
        // edit rather than one merged run.
        let mut txn = doc.transact_mut();
        match edit {
            Edit::Insert { at, ch } => {
                let mut buffer = [0u8; 4];
                text.insert(&mut txn, at as u32, ch.encode_utf8(&mut buffer));
            }
            Edit::Delete { at } => text.remove_range(&mut txn, at as u32, 1),
        }
    }
    let chars = {
        let txn = doc.transact();
        text.get_string(&txn).chars().count()
    };
    let full = doc
        .transact()
        .encode_state_as_update_v1(&StateVector::default())
        .len();
    format!(
        "{chars} chars, full {full} bytes, trimmed n/a: garbage collection removes \
         deleted content and does not drop a history prefix"
    )
}
