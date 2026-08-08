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

//! The committed fixtures for the CSV importer, and the result each one is
//! expected to produce.
//!
//! # Why the fixtures are byte literals and not files
//!
//! Every case below turns on bytes that a checkout can change. A file holding
//! `\r\n` line endings, a file holding a byte order mark and a file holding
//! bytes that are not UTF-8 are all things git will normalise or a text editor
//! will helpfully repair, and the repair is silent. This repository has no
//! `.gitattributes`, so whether a tracked text file arrives with its carriage
//! returns intact is a property of each clone's own `core.autocrlf` setting,
//! which no tree can read and no reviewer can see.
//!
//! A byte-string literal with explicit escapes carries none of that risk. The
//! source file is plain ASCII whatever the clone does with it, and the bytes
//! the parser is handed are the bytes written here. The fixture is committed;
//! it is committed as an escape sequence rather than as a file, which is what
//! makes it exact.
//!
//! Each literal is one line for the same reason: a literal broken across lines
//! would carry this file's own line endings into the fixture, and those are the
//! ones the clone decides.

use std::io::{Cursor, Read};
use std::sync::atomic::{AtomicU64, Ordering};

use kartei_import::csv::{
    Column, ColumnType, DateFormat, Encoding, Flag, FlagReason, ImportError, Limits, Question,
    Source, Survey, Table, Value,
};

/// Reads a fixture under the default budgets.
fn read(bytes: &[u8]) -> Source {
    Source::read(&mut Cursor::new(bytes), &Limits::default())
        .unwrap_or_else(|e| panic!("the fixture did not read: {e}"))
}

/// Reads a fixture and imports it under the plan the importer inferred.
fn survey_and_import(bytes: &[u8]) -> (Survey, Table) {
    let source = read(bytes);
    let survey = source.survey();
    let table = source
        .import(&survey.plan)
        .unwrap_or_else(|e| panic!("the fixture did not import: {e}"));
    (survey, table)
}

/// The column types of a survey, which is what most of these assertions are
/// about.
fn kinds(survey: &Survey) -> Vec<ColumnType> {
    survey.plan.columns.iter().map(|c| c.kind).collect()
}

/// The column names of a survey.
fn names(survey: &Survey) -> Vec<String> {
    survey.plan.columns.iter().map(|c| c.name.clone()).collect()
}

/// Text, as a value.
fn text(s: &str) -> Value {
    Value::Text(s.to_owned())
}

/// A number, as a value.
fn number(s: &str) -> Value {
    Value::Number(s.to_owned())
}

/// A date, as a value.
fn date(year: i32, month: u32, day: u32) -> Value {
    Value::Date { year, month, day }
}

// ---------------------------------------------------------------------------
// Dialect: delimiter, quoting and encoding are detected rather than assumed.
// ---------------------------------------------------------------------------

/// The ordinary case, and the one every other fixture is a departure from.
#[test]
fn a_comma_separated_utf8_file_reads_as_one_table() {
    let fixture = b"name,quantity,active,ordered\nada,12,yes,2024-03-05\ngrace,7,no,2024-11-30\n";

    let (survey, table) = survey_and_import(fixture);

    assert_eq!(survey.dialect.delimiter, ',');
    assert_eq!(survey.dialect.encoding, Encoding::Utf8);
    assert_eq!(survey.rows, 2);
    assert_eq!(names(&survey), ["name", "quantity", "active", "ordered"]);
    assert_eq!(
        kinds(&survey),
        [
            ColumnType::Text,
            ColumnType::Number,
            ColumnType::Checkbox,
            ColumnType::Date(Some(DateFormat::Iso)),
        ]
    );
    assert_eq!(
        table.rows,
        vec![
            vec![
                text("ada"),
                number("12"),
                Value::Checkbox(true),
                date(2024, 3, 5)
            ],
            vec![
                text("grace"),
                number("7"),
                Value::Checkbox(false),
                date(2024, 11, 30)
            ],
        ]
    );
    assert!(table.flags.is_empty(), "flags: {:?}", table.flags);
}

/// The separator a spreadsheet writes wherever the comma is the decimal point.
#[test]
fn a_semicolon_separated_file_is_detected_rather_than_read_as_one_column() {
    let fixture = b"name;quantity\nada;12\ngrace;7\n";

    let (survey, table) = survey_and_import(fixture);

    assert_eq!(survey.dialect.delimiter, ';');
    assert_eq!(names(&survey), ["name", "quantity"]);
    assert_eq!(
        table.rows,
        vec![
            vec![text("ada"), number("12")],
            vec![text("grace"), number("7")],
        ]
    );
}

/// A tab-separated export, which is what several of the incumbents produce.
#[test]
fn a_tab_separated_file_is_detected() {
    let fixture = b"name\tquantity\nada\t12\ngrace\t7\n";

    let survey = read(fixture).survey();

    assert_eq!(survey.dialect.delimiter, '\t');
    assert_eq!(names(&survey), ["name", "quantity"]);
}

/// A file with one column has no delimiter to find, and this states what it
/// does instead of leaving it to be discovered.
#[test]
fn a_single_column_file_falls_back_to_a_comma_and_reads_as_one_column() {
    let fixture = b"name\nada\ngrace\n";

    let survey = read(fixture).survey();

    assert_eq!(survey.dialect.delimiter, ',');
    assert_eq!(names(&survey), ["name"]);
}

/// A quoted field holds the delimiter, a line ending and a quote, and none of
/// the three ends the field.
#[test]
fn quoting_covers_the_delimiter_the_line_ending_and_the_quote_itself() {
    let fixture =
        b"name,note\n\"Ada, Countess\",\"line one\nline two\"\n\"Grace\",\"she said \"\"hello\"\"\"\n";

    let (survey, table) = survey_and_import(fixture);

    assert_eq!(survey.rows, 2);
    assert_eq!(
        table.rows,
        vec![
            vec![text("Ada, Countess"), text("line one\nline two")],
            vec![text("Grace"), text("she said \"hello\"")],
        ]
    );
}

/// Carriage returns, which is what most of these files actually carry.
#[test]
fn carriage_return_line_endings_read_the_same_as_bare_ones() {
    let fixture = b"name,quantity\r\nada,12\r\ngrace,7\r\n";

    let (survey, table) = survey_and_import(fixture);

    assert_eq!(survey.rows, 2);
    assert_eq!(
        table.rows,
        vec![
            vec![text("ada"), number("12")],
            vec![text("grace"), number("7")],
        ]
    );
}

/// The mark a spreadsheet on Windows writes, which must not become part of the
/// first column's name.
#[test]
fn a_utf8_byte_order_mark_is_consumed_rather_than_becoming_a_column_name() {
    let fixture = b"\xEF\xBB\xBFname,quantity\nada,12\n";

    let survey = read(fixture).survey();

    assert_eq!(survey.dialect.encoding, Encoding::Utf8Bom);
    assert_eq!(names(&survey), ["name", "quantity"]);
}

/// UTF-16, little endian, by its mark. The literal is spelled out byte by byte
/// because that is the thing under test.
#[test]
fn a_utf16_little_endian_file_decodes_by_its_mark() {
    // FF FE, then "a,b\n1,\u{00E9}\n" as little-endian code units.
    let fixture = b"\xFF\xFE\x61\x00\x2C\x00\x62\x00\x0A\x00\x31\x00\x2C\x00\xE9\x00\x0A\x00";

    let (survey, table) = survey_and_import(fixture);

    assert_eq!(survey.dialect.encoding, Encoding::Utf16Le);
    assert_eq!(names(&survey), ["a", "b"]);
    assert_eq!(table.rows, vec![vec![number("1"), text("\u{00E9}")]]);
}

/// The same file cut in the middle of a code unit. The mark is a declaration,
/// so this is refused rather than read some other way.
#[test]
fn a_utf16_file_that_ends_mid_unit_is_refused() {
    let fixture = b"\xFF\xFE\x61\x00\x2C\x00\x62\x00\x0A\x00\x31";

    let error = Source::read(&mut Cursor::new(fixture), &Limits::default())
        .expect_err("a truncated UTF-16 file should not read");

    assert_eq!(error, ImportError::TruncatedUtf16);
}

/// Not valid UTF-8 and carrying no mark, which is most of the CSV files that
/// are not UTF-8.
#[test]
fn a_file_that_is_not_utf8_and_has_no_mark_reads_as_windows_1252() {
    // 0xE9 is a lone continuation byte in UTF-8 and is `e` with an acute accent
    // in windows-1252. 0x92 is undefined in Latin-1 and is a right single
    // quotation mark in windows-1252, which is the pair that tells the two
    // encodings apart.
    let fixture = b"name,note\nada,caf\xE9\ngrace,it\x92s fine\n";

    let (survey, table) = survey_and_import(fixture);

    assert_eq!(survey.dialect.encoding, Encoding::Windows1252);
    assert_eq!(
        table.rows,
        vec![
            vec![text("ada"), text("caf\u{00E9}")],
            vec![text("grace"), text("it\u{2019}s fine")],
        ]
    );
}

// ---------------------------------------------------------------------------
// Inference, and the one case it refuses to decide.
// ---------------------------------------------------------------------------

/// Both readings are complete, so the importer asks.
#[test]
fn an_ambiguous_date_column_produces_a_question_rather_than_a_silent_choice() {
    let fixture = b"name,due\nada,05/03/2024\ngrace,06/04/2024\n";

    let source = read(fixture);
    let survey = source.survey();

    assert_eq!(kinds(&survey), [ColumnType::Text, ColumnType::Date(None)]);
    assert_eq!(
        survey.plan.columns[1].question,
        Some(Question::DateOrder {
            examples: vec!["05/03/2024".to_owned(), "06/04/2024".to_owned()],
        })
    );
    assert_eq!(survey.unanswered(), vec![1]);
}

/// The question is a refusal and not a note. This is the assertion that would
/// go red if the importer ever started picking an order.
#[test]
fn an_unanswered_date_question_refuses_the_import_entirely() {
    let fixture = b"name,due\nada,05/03/2024\ngrace,06/04/2024\n";

    let source = read(fixture);
    let survey = source.survey();

    let error = source
        .import(&survey.plan)
        .expect_err("an unanswered question should refuse the import");

    assert_eq!(
        error,
        ImportError::UnansweredQuestion {
            column: 1,
            name: "due".to_owned(),
        }
    );
}

/// The two answers, and what each one does to the same bytes. Together these
/// are why the question exists: the same file is two different tables.
#[test]
fn answering_the_date_question_is_what_decides_the_dates() {
    let fixture = b"name,due\nada,05/03/2024\ngrace,06/04/2024\n";

    let source = read(fixture);

    for (answer, expected) in [
        (
            DateFormat::DayFirst,
            vec![date(2024, 3, 5), date(2024, 4, 6)],
        ),
        (
            DateFormat::MonthFirst,
            vec![date(2024, 5, 3), date(2024, 6, 4)],
        ),
    ] {
        let mut plan = source.survey().plan;
        plan.columns[1].kind = ColumnType::Date(Some(answer));

        let table = source
            .import(&plan)
            .unwrap_or_else(|e| panic!("an answered plan should import: {e}"));

        let got: Vec<Value> = table.rows.iter().map(|r| r[1].clone()).collect();
        assert_eq!(got, expected, "under {answer:?}");
    }
}

/// A part above twelve can only be a day, so there is nothing to ask.
#[test]
fn a_value_above_twelve_in_the_first_slot_settles_the_order_as_day_first() {
    let fixture = b"name,due\nada,13/03/2024\ngrace,05/04/2024\n";

    let (survey, table) = survey_and_import(fixture);

    assert_eq!(
        survey.plan.columns[1].kind,
        ColumnType::Date(Some(DateFormat::DayFirst))
    );
    assert_eq!(survey.plan.columns[1].question, None);
    assert_eq!(
        table.rows.iter().map(|r| r[1].clone()).collect::<Vec<_>>(),
        vec![date(2024, 3, 13), date(2024, 4, 5)]
    );
}

/// The mirror of the case above.
#[test]
fn a_value_above_twelve_in_the_second_slot_settles_the_order_as_month_first() {
    let fixture = b"name,due\nada,03/13/2024\ngrace,04/05/2024\n";

    let (survey, table) = survey_and_import(fixture);

    assert_eq!(
        survey.plan.columns[1].kind,
        ColumnType::Date(Some(DateFormat::MonthFirst))
    );
    assert_eq!(
        table.rows.iter().map(|r| r[1].clone()).collect::<Vec<_>>(),
        vec![date(2024, 3, 13), date(2024, 4, 5)]
    );
}

/// The near-miss: one row can only be day-first and another can only be
/// month-first, so no order reads the column. Choosing either would move real
/// dates, so the column stays text and every byte survives.
#[test]
fn a_column_that_contradicts_itself_is_text_rather_than_half_a_date_column() {
    let fixture = b"name,due\nada,13/03/2024\ngrace,03/13/2024\n";

    let (survey, table) = survey_and_import(fixture);

    assert_eq!(survey.plan.columns[1].kind, ColumnType::Text);
    assert_eq!(survey.plan.columns[1].question, None);
    assert_eq!(
        table.rows.iter().map(|r| r[1].clone()).collect::<Vec<_>>(),
        vec![text("13/03/2024"), text("03/13/2024")]
    );
}

/// A two-digit year needs a century supplied from outside the file, so it is
/// not a date here.
#[test]
fn a_two_digit_year_is_not_inferred_as_a_date() {
    let fixture = b"name,due\nada,05/03/24\ngrace,06/04/24\n";

    let survey = read(fixture).survey();

    assert_eq!(survey.plan.columns[1].kind, ColumnType::Text);
}

/// A thousands separator is locale-dependent, so a column carrying one is not
/// read as a number.
#[test]
fn a_thousands_separator_leaves_the_column_as_text() {
    let fixture = b"name,amount\nada,1,234\n";

    // The separator detection sees three fields per record, which is the honest
    // reading of these bytes: a comma-separated file cannot also use the comma
    // as a thousands separator, and this fixture records that rather than
    // pretending the importer can tell them apart.
    let survey = read(fixture).survey();
    assert_eq!(survey.plan.columns.len(), 2);

    // The same content with a separator that does not collide.
    let fixture = b"name;amount\nada;1,234\ngrace;2,000\n";
    let survey = read(fixture).survey();

    assert_eq!(survey.dialect.delimiter, ';');
    assert_eq!(survey.plan.columns[1].kind, ColumnType::Text);
}

/// Not the date shape but the date arithmetic: `2024-02-31` looks like a date
/// and is not one.
#[test]
fn a_day_that_does_not_exist_stops_the_column_being_a_date() {
    let fixture = b"name,ordered\nada,2024-02-31\ngrace,2024-03-05\n";

    let survey = read(fixture).survey();

    assert_eq!(survey.plan.columns[1].kind, ColumnType::Text);
}

// ---------------------------------------------------------------------------
// Empty, zero and the empty string.
// ---------------------------------------------------------------------------

/// Three cells that are routinely collapsed into one, kept apart.
#[test]
fn nothing_zero_and_an_empty_string_are_three_different_cells() {
    let fixture = b"name,amount,note\nada,,alpha\ngrace,0,\nalan,7,\"\"\n";

    let (survey, table) = survey_and_import(fixture);

    // The amount column has numbers and a gap, and a gap is not evidence about
    // the type, so it is still a number column.
    assert_eq!(survey.plan.columns[1].kind, ColumnType::Number);
    // The note column holds an explicit empty string, which is a value and not
    // a gap, so the column is text.
    assert_eq!(survey.plan.columns[2].kind, ColumnType::Text);

    assert_eq!(
        table.rows,
        vec![
            vec![text("ada"), Value::Empty, text("alpha")],
            vec![text("grace"), number("0"), Value::Empty],
            vec![text("alan"), number("7"), text("")],
        ]
    );
    assert!(table.flags.is_empty(), "flags: {:?}", table.flags);
}

// ---------------------------------------------------------------------------
// Nothing is dropped and nothing is coerced.
// ---------------------------------------------------------------------------

/// A correction is what creates values that do not fit, because inference
/// cannot: a column is only judged a number if every value in it is one. The
/// values that do not fit are kept exactly and flagged.
#[test]
fn a_value_that_does_not_fit_a_corrected_column_is_kept_as_written_and_flagged() {
    let fixture = b"name,quantity\nada,12\ngrace,none\nalan,7\n";

    let source = read(fixture);
    let survey = source.survey();
    assert_eq!(
        survey.plan.columns[1].kind,
        ColumnType::Text,
        "one value that is not a number is enough to make the column text"
    );

    let mut plan = survey.plan;
    plan.columns[1].kind = ColumnType::Number;

    let table = source
        .import(&plan)
        .unwrap_or_else(|e| panic!("a corrected plan should import: {e}"));

    assert_eq!(
        table.rows,
        vec![
            vec![text("ada"), number("12")],
            vec![text("grace"), Value::Raw("none".to_owned())],
            vec![text("alan"), number("7")],
        ]
    );
    assert_eq!(
        table.flags,
        vec![Flag {
            row: 1,
            column: Some(1),
            reason: FlagReason::NotOfColumnType,
            value: "none".to_owned(),
        }]
    );
}

/// A short row is padded and a long row keeps its extra fields past the end of
/// the row. Neither loses a byte and both are flagged.
#[test]
fn rows_that_do_not_match_the_header_are_flagged_and_nothing_is_discarded() {
    let fixture = b"a,b,c\n1,2,3\n4,5\n6,7,8,9\n";

    let (_, table) = survey_and_import(fixture);

    assert_eq!(
        table.rows,
        vec![
            vec![number("1"), number("2"), number("3")],
            vec![number("4"), number("5"), Value::Empty],
            vec![
                number("6"),
                number("7"),
                number("8"),
                Value::Raw("9".to_owned())
            ],
        ]
    );
    assert_eq!(
        table.flags,
        vec![
            Flag {
                row: 1,
                column: None,
                reason: FlagReason::ShortRow,
                value: "2 field(s) against 3 column(s)".to_owned(),
            },
            Flag {
                row: 2,
                column: None,
                reason: FlagReason::LongRow,
                value: "4 field(s) against 3 column(s)".to_owned(),
            },
        ]
    );
}

// ---------------------------------------------------------------------------
// Bounds.
// ---------------------------------------------------------------------------

/// A reader that never ends, which is the shape of the failure the byte budget
/// exists against.
struct Endless {
    served: AtomicU64,
}

impl Read for Endless {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        buf.fill(b'a');
        self.served.fetch_add(buf.len() as u64, Ordering::Relaxed);
        Ok(buf.len())
    }
}

/// The budget is a refusal, and it is enforced while reading rather than after.
///
/// The second assertion is the one that matters. A budget checked after the
/// file is in memory refuses the same input and has already done the damage,
/// and against this reader it would never return at all.
#[test]
fn a_file_over_the_byte_budget_is_refused_without_being_read_past_the_budget() {
    let mut endless = Endless {
        served: AtomicU64::new(0),
    };
    let limits = Limits {
        max_bytes: 4096,
        max_rows: 1_000_000,
    };

    let error = Source::read(&mut endless, &limits)
        .expect_err("an endless reader should be refused by the byte budget");

    assert_eq!(error, ImportError::TooLarge { limit: 4096 });
    assert_eq!(
        endless.served.load(Ordering::Relaxed),
        4097,
        "the reader was asked for more than one byte past the budget"
    );
}

/// A file inside the byte budget can still hold more rows than the row budget.
#[test]
fn a_file_over_the_row_budget_is_refused_rather_than_imported_in_part() {
    let mut fixture = b"name,quantity\n".to_vec();
    for i in 0..10u32 {
        fixture.extend_from_slice(format!("row{i},{i}\n").as_bytes());
    }
    let limits = Limits {
        max_bytes: 64 * 1024,
        max_rows: 3,
    };

    let error = Source::read(&mut Cursor::new(&fixture), &limits)
        .expect_err("eleven rows should be refused by a budget of three");

    assert_eq!(error, ImportError::TooManyRows { limit: 3 });

    // The same file under a budget that fits reads all ten rows, so the refusal
    // above is attributable to the budget and not to the fixture.
    let limits = Limits {
        max_bytes: 64 * 1024,
        max_rows: 10,
    };
    let source = Source::read(&mut Cursor::new(&fixture), &limits)
        .unwrap_or_else(|e| panic!("ten rows under a budget of ten should read: {e}"));
    assert_eq!(source.survey().rows, 10);
}

/// A file with nothing in it has no header, and a table with no columns is not
/// something to import quietly.
#[test]
fn a_file_with_no_records_is_refused_rather_than_imported_as_an_empty_table() {
    let error = Source::read(&mut Cursor::new(b""), &Limits::default())
        .expect_err("an empty file should not produce a table");

    assert_eq!(error, ImportError::NoHeader);
}

/// Every error carries a sentence an operator can act on. A refusal whose
/// message names nothing is a refusal they have to guess about.
#[test]
fn every_refusal_says_what_it_refused_and_why() {
    let cases = [
        ImportError::TooLarge { limit: 4096 },
        ImportError::TooManyRows { limit: 3 },
        ImportError::NoHeader,
        ImportError::InvalidUtf8,
        ImportError::TruncatedUtf16,
        ImportError::InvalidUtf16,
        ImportError::UnansweredQuestion {
            column: 1,
            name: "due".to_owned(),
        },
        ImportError::Read {
            message: "a message".to_owned(),
        },
    ];

    for case in cases {
        let message = case.to_string();
        assert!(
            message.len() > 30 && message.chars().any(char::is_alphabetic),
            "{case:?} produced {message:?}, which tells an operator nothing"
        );
    }
}

/// The blank lines a spreadsheet leaves at the end of a file are not rows.
#[test]
fn blank_lines_do_not_become_rows_of_empty_cells() {
    let fixture = b"name,quantity\nada,12\n\ngrace,7\n\n";

    let (survey, table) = survey_and_import(fixture);

    assert_eq!(survey.rows, 2);
    assert_eq!(table.rows.len(), 2);
}

/// The survey is a value the caller can hold, edit and pass back, which is what
/// makes the inference correctable at all.
#[test]
fn a_plan_can_be_edited_and_applied_without_reading_the_file_again() {
    let fixture = b"reference,quantity\n007,12\n013,7\n";

    let source = read(fixture);
    let survey = source.survey();

    // Leading zeroes make this look like a number and it is an order reference,
    // which is the case the two-phase shape exists for.
    assert_eq!(survey.plan.columns[0].kind, ColumnType::Number);

    let mut plan = survey.plan;
    plan.columns[0] = Column {
        name: "reference".to_owned(),
        kind: ColumnType::Text,
        question: None,
    };

    let table = source
        .import(&plan)
        .unwrap_or_else(|e| panic!("a corrected plan should import: {e}"));

    assert_eq!(
        table.rows.iter().map(|r| r[0].clone()).collect::<Vec<_>>(),
        vec![text("007"), text("013")]
    );
}
