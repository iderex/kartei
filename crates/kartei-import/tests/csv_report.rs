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

//! What the CSV import reports about itself.
//!
//! The importer's own behaviour is covered in `csv_fixtures.rs`. This file is
//! about the account it gives of that behaviour, which is a separate thing that
//! can be wrong on its own: an importer that does exactly the right thing and
//! describes it wrongly sends somebody looking in the wrong place.
//!
//! Every fixture here is bytes in the test rather than a file on disk, so the
//! suite needs no fixture tree and no path, and the bytes that reach the parser
//! are the bytes written here.
//!
//! The near-misses are half of this file on purpose. A report that names a
//! construct the file does not have is worse than one that names nothing,
//! because it is acted on.

use kartei_import::csv::{ColumnType, Plan, Source, Table};
use kartei_import::report::{Report, SITES_SHOWN, State};

/// The budget is not what any of this is about, so it is out of the way.
fn read(bytes: &[u8]) -> Source {
    Source::read(&mut &bytes[..], &kartei_import::csv::Limits::default())
        .expect("the fixture is meant to be readable")
}

/// Read, survey and import under the inferred plan.
fn import(bytes: &[u8]) -> Table {
    let source = read(bytes);
    let plan = source.survey().plan;
    source.import(&plan).expect("the fixture asks no question")
}

/// Read and import under a plan the caller has edited, which is how a
/// correction reaches the importer.
fn import_with(bytes: &[u8], edit: impl FnOnce(&mut Plan)) -> Table {
    let source = read(bytes);
    let mut plan = source.survey().plan;
    edit(&mut plan);
    source.import(&plan).expect("the fixture asks no question")
}

/// The constructs one state names, in the order the report holds them.
fn named(report: &Report, state: State) -> Vec<String> {
    report
        .in_state(state)
        .map(|e| e.construct.clone())
        .collect()
}

// ---------------------------------------------------------------------------
// An import that skipped nothing says so.
// ---------------------------------------------------------------------------

/// The negative answer is written out rather than left as an absence, because
/// an empty report and a report from an importer that stopped looking are the
/// same bytes otherwise.
#[test]
fn an_import_that_skipped_nothing_says_so_in_words() {
    let table = import(b"name,count\nada,1\ngrace,2\n");

    assert!(
        table.report.nothing_was_skipped(),
        "this fixture has nothing to skip and nothing to degrade, and the report \
         says otherwise: {}",
        table.report
    );
    assert!(
        named(&table.report, State::NotSupported).is_empty(),
        "a clean file produced an unsupported construct"
    );
    assert!(
        named(&table.report, State::Degraded).is_empty(),
        "a clean file produced a degradation"
    );

    let rendered = table.report.to_string();
    assert!(
        rendered.contains("Nothing was skipped and nothing was carried in a degraded form."),
        "the rendered report leaves the reader to infer the negative:\n{rendered}"
    );
}

/// Every heading is printed even when it holds nothing, so a state with no
/// entries reads as looked-at rather than as not looked at.
#[test]
fn every_state_has_a_heading_in_the_rendered_report() {
    let rendered = import(b"name,count\nada,1\n").report.to_string();

    for state in [State::NotInSource, State::NotSupported, State::Degraded] {
        assert!(
            rendered.contains(state.heading()),
            "the rendered report has no heading for {state:?}:\n{rendered}"
        );
    }
    assert!(
        rendered.contains("  none"),
        "a state with no entries printed nothing at all:\n{rendered}"
    );
}

// ---------------------------------------------------------------------------
// The three states are distinct.
// ---------------------------------------------------------------------------

/// One file that produces all three, so that the separation is exercised rather
/// than asserted about the type.
#[test]
fn the_three_states_are_separate_in_one_report() {
    // Column 1 is thousands-separated, which this importer declines to read as
    // numbers. The last row is short, which is a degradation. A delimited file
    // carries no formula, which is the third.
    let table = import(b"name;amount\nada;1,234\ngrace\n");

    assert_eq!(
        named(&table.report, State::NotInSource),
        vec!["formula", "cell formatting"],
        "the report changed what a delimited file cannot carry"
    );
    assert_eq!(
        named(&table.report, State::NotSupported),
        vec!["thousands separator"],
    );
    assert_eq!(
        named(&table.report, State::Degraded),
        vec!["row shorter than the header"],
    );

    assert!(
        !table.report.nothing_was_skipped(),
        "something was skipped and the report says nothing was"
    );
}

/// Something the export never carried is not something this importer skipped,
/// and folding the two together would make every import look lossy.
#[test]
fn a_construct_absent_from_the_format_does_not_count_as_skipped() {
    let table = import(b"name,count\nada,1\n");

    assert!(
        table.report.names("formula"),
        "the report says nothing about formulas, which is the question people arrive with"
    );
    assert!(
        table.report.nothing_was_skipped(),
        "an entry about what the format cannot carry was counted as a skip"
    );
}

// ---------------------------------------------------------------------------
// A known unsupported construct is named.
// ---------------------------------------------------------------------------

/// The guess the module refuses to make, and the sentence a person needs.
///
/// Without this the column arrives as text and the report says nothing, which
/// leaves somebody to work out from the data why their amounts will not sum.
#[test]
fn a_thousands_separated_column_is_named_as_unsupported() {
    let table = import(b"name;amount\nada;1,234\ngrace;2,000\n");

    assert_eq!(table.columns[1].kind, ColumnType::Text);
    assert!(
        table.report.names("thousands separator"),
        "the column came in as text and the report did not say why:\n{}",
        table.report
    );

    let entry = table
        .report
        .in_state(State::NotSupported)
        .find(|e| e.construct == "thousands separator")
        .expect("just asserted it is there");
    assert_eq!(entry.sites, vec!["column 1, amount".to_owned()]);
}

/// The other declined guess, for the same reason.
#[test]
fn a_two_digit_year_column_is_named_as_unsupported() {
    let table = import(b"name,due\nada,05/03/24\ngrace,06/04/24\n");

    assert_eq!(table.columns[1].kind, ColumnType::Text);
    assert!(
        table.report.names("two-digit year"),
        "the column came in as text and the report did not say why:\n{}",
        table.report
    );
}

/// Grouping written with a full stop rather than a comma, which is the same
/// construct in another locale and must not be a different answer.
#[test]
fn grouping_is_recognised_whichever_mark_writes_it() {
    let table = import(b"name;amount\nada;1.234,50\ngrace;2.000,00\n");

    assert_eq!(table.columns[1].kind, ColumnType::Text);
    assert!(table.report.names("thousands separator"));
}

// ---------------------------------------------------------------------------
// Near-misses. A report that names a construct the file does not have is acted
// on and is wrong.
// ---------------------------------------------------------------------------

/// Text that is text. The commonest column in any file, and the one a
/// too-eager detector would report on.
#[test]
fn a_column_of_ordinary_text_is_not_reported_as_a_declined_guess() {
    let table = import(b"name,note\nada,first\ngrace,second\n");

    assert!(
        named(&table.report, State::NotSupported).is_empty(),
        "a column of plain text was reported as a construct:\n{}",
        table.report
    );
}

/// One value out of a column of grouped numbers, so the column is not a column
/// of grouped numbers. The one-character version of the mistake: a detector
/// written with `any` instead of `all` passes every other test in this file and
/// fails this one.
#[test]
fn a_single_odd_value_stops_the_column_being_claimed() {
    let table = import(b"name;amount\nada;1,234\ngrace;pending\n");

    assert!(
        !table.report.names("thousands separator"),
        "a column holding a word was claimed as thousands-separated numbers:\n{}",
        table.report
    );
}

/// A plain number column is a number column, and nothing in the report should
/// suggest a construct was declined.
#[test]
fn a_plain_number_column_produces_no_declined_guess() {
    let table = import(b"name,amount\nada,1234\ngrace,2000.50\n");

    assert_eq!(table.columns[1].kind, ColumnType::Number);
    assert!(named(&table.report, State::NotSupported).is_empty());
}

/// A four-digit year is read as a date, so there is no declined guess to
/// report. The neighbour of the two-digit case, one character away.
#[test]
fn a_four_digit_year_column_is_a_date_and_reports_nothing() {
    let table = import(b"name,due\nada,05/03/2024\ngrace,25/04/2024\n");

    assert!(matches!(table.columns[1].kind, ColumnType::Date(Some(_))));
    assert!(named(&table.report, State::NotSupported).is_empty());
}

// ---------------------------------------------------------------------------
// Degradations, and where they happened.
// ---------------------------------------------------------------------------

/// A cell that did not fit a corrected column. The value survives in the table
/// and the report says which cell to look at.
#[test]
fn a_cell_outside_its_column_type_is_a_degradation_naming_the_cell() {
    let table = import_with(b"name,amount\nada,1\ngrace,later\n", |plan| {
        plan.columns[1].kind = ColumnType::Number;
    });

    let entry = table
        .report
        .in_state(State::Degraded)
        .find(|e| e.construct == "value outside its column's type")
        .unwrap_or_else(|| panic!("no degradation was reported:\n{}", table.report));

    assert_eq!(entry.count, 1);
    assert_eq!(entry.sites, vec!["row 1, column 1".to_owned()]);
}

/// A row longer than the header. Nothing is discarded, and the report says so
/// and says which row.
#[test]
fn a_long_row_is_a_degradation_naming_the_row() {
    let table = import(b"name,count\nada,1\ngrace,2,extra\n");

    let entry = table
        .report
        .in_state(State::Degraded)
        .find(|e| e.construct == "row longer than the header")
        .unwrap_or_else(|| panic!("no degradation was reported:\n{}", table.report));

    assert_eq!(entry.sites, vec!["row 1".to_owned()]);
}

/// The count is exact and the list is bounded, and the difference between the
/// two is printed rather than left to be noticed.
#[test]
fn many_occurrences_are_counted_exactly_and_the_truncation_is_disclosed() {
    let mut fixture = b"name,count\n".to_vec();
    let occurrences = SITES_SHOWN + 5;
    for i in 0..occurrences {
        fixture.extend_from_slice(format!("row{i},1,extra\n").as_bytes());
    }

    let table = import(&fixture);
    let entry = table
        .report
        .in_state(State::Degraded)
        .find(|e| e.construct == "row longer than the header")
        .unwrap_or_else(|| panic!("no degradation was reported:\n{}", table.report));

    assert_eq!(entry.count, occurrences, "the count is not the exact one");
    assert_eq!(entry.sites.len(), SITES_SHOWN, "the list is not bounded");
    assert_eq!(entry.unnamed(), 5);

    let rendered = table.report.to_string();
    assert!(
        rendered.contains("and 5 more, not listed"),
        "the rendered report trails off instead of saying how many it did not \
         name:\n{rendered}"
    );
}

// ---------------------------------------------------------------------------
// The report travels with the table.
// ---------------------------------------------------------------------------

/// The account outlives the thing that produced it, so whatever stores the
/// table can store the account beside it. Dropping the `Source` is the test:
/// a report borrowed from the parse would not survive it.
#[test]
fn the_report_outlives_the_source_it_came_from() {
    let table = {
        let source = read(b"name;amount\nada;1,234\n");
        let plan = source.survey().plan;
        source.import(&plan).expect("the fixture asks no question")
    };

    assert_eq!(table.report.importer, "csv");
    assert_eq!(table.report.counts.columns, 2);
    assert_eq!(table.report.counts.rows, 1);
    assert!(table.report.names("thousands separator"));
}

/// The counts describe the table that came back, so a reader comparing them
/// against the table is comparing two views of one import.
#[test]
fn the_counts_describe_the_table_they_arrived_with() {
    let table = import(b"name,count\nada,1\ngrace,2\nhopper,3\n");

    assert_eq!(table.report.counts.columns, table.columns.len());
    assert_eq!(table.report.counts.rows, table.rows.len());
    assert_eq!(
        table.report.counts.cells,
        table.rows.iter().map(Vec::len).sum::<usize>()
    );
}
