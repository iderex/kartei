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

//! Reading a table out of a delimited text file.
//!
//! This is the building block the other importers stand on, and it is also a
//! feature on its own, because a table exported from a spreadsheet or from an
//! existing base is how most tables arrive.
//!
//! # Two phases, because inference has to be correctable
//!
//! [`Source::read`] decodes and parses. [`Source::survey`] reports what it
//! thinks each column is, and [`Source::import`] applies a [`Plan`] the caller
//! may have edited first. Inference that cannot be corrected is worse than no
//! inference: it is a guess wearing the clothes of a fact, and the operator
//! finds out months later, when the column that was silently read as a number
//! turns out to have been an order reference with leading zeroes.
//!
//! # What is guessed and what is refused
//!
//! Delimiter, quoting and encoding are detected rather than assumed, because
//! assuming them means refusing most of the CSV files that exist. Where a guess
//! would be a coin toss the importer asks instead: a column of dates whose
//! parts are all twelve or below is day-first under one reading and month-first
//! under another, both readings are complete and self-consistent, and picking
//! one silently turns the fifth of March into the third of May in a way nothing
//! downstream can detect. That case becomes a [`Question`] on the survey, and
//! [`Source::import`] refuses to run until it is answered.
//!
//! Two other guesses are deliberately not made, for the same reason and with
//! the same consequence of being read as text. A thousands separator is
//! locale-dependent, so `1.234` is one number in one place and another number
//! somewhere else, and a column carrying them stays [`ColumnType::Text`]. A
//! two-digit year needs a century, and the rule for supplying one is a
//! convention rather than a fact, so `05/03/24` is not a date here.
//!
//! # Empty, zero and the empty string are three things
//!
//! A field with no bytes at all is [`Value::Empty`]: nothing was written. A
//! field written as `""` is an empty string: something was written and it was
//! empty. A field written as `0` is a number. Collapsing any two of them loses
//! a distinction the source file made deliberately, and a column that holds an
//! explicit empty string is therefore text rather than a number, which is the
//! honest reading rather than a convenient one.
//!
//! # Nothing is dropped and nothing is coerced
//!
//! A value that does not fit the column's type is kept exactly as it was
//! written, as [`Value::Raw`], and a [`Flag`] records where it was. A row with
//! more fields than the header has keeps the extra fields past the end of the
//! row. A row with fewer is padded with [`Value::Empty`]. In every case the
//! bytes survive and the flag says so, because an importer that quietly drops
//! what it did not understand is how an import becomes data loss.
//!
//! # The account of what happened
//!
//! [`Table::report`] is that account, in the shared shape every importer owes,
//! and [`crate::report`] is where the three states it separates are argued.
//! Two of them are produced here from things this module already knew and did
//! not say out loud. A [`Flag`] becomes a degradation, because a cell kept as
//! written under a column that is not its type is a cell that arrived and lost
//! its type. A guess this importer declines to make becomes an unsupported
//! construct, because "your column of thousands-separated numbers came in as
//! text" and "your column came in as text" are different sentences and only the
//! first one can be acted on.
//!
//! # Bounds
//!
//! [`Limits`] carries a byte budget and a row budget, and both refuse rather
//! than truncate. The budget is enforced while reading, through
//! [`Read::take`], so a file far larger than the budget is refused having
//! occupied one byte more than the budget rather than all of itself. That
//! matters here more than it would elsewhere: the deployment this product is
//! for is one process on a box that is already running something else, so an
//! import that exhausts memory takes the workspace down with it.
//!
//! # What this module does not decide
//!
//! The field types of the product are #35's, and are not defined yet.
//! [`ColumnType`] is this importer's own judgement about what a column of text
//! looks like, and mapping it onto the model is that issue's work rather than
//! this one's. That is also why a number is carried as the lexeme that was
//! validated rather than as a machine number: converting before anybody has
//! chosen the target representation would throw away precision that the file
//! contained and that nothing could get back.

use std::fmt;
use std::io::Read;

use crate::report::{Counts, Entry, Report, State};

/// The bounds an import is held to.
///
/// Both are refusals rather than truncations. Silently importing the first part
/// of a file gives the operator a table that looks complete and is not, which
/// is worse than an error they can act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The largest input, in bytes, that will be read at all.
    pub max_bytes: u64,
    /// The largest number of data rows, not counting the header row.
    pub max_rows: usize,
}

impl Default for Limits {
    /// Sixty-four mebibytes and a million rows.
    ///
    /// Both are starting positions rather than measurements: no import has been
    /// profiled yet, so these are the sizes past which a single-process
    /// deployment should be asked rather than assumed. The operator-facing
    /// setting is M8's.
    fn default() -> Limits {
        Limits {
            max_bytes: 64 * 1024 * 1024,
            max_rows: 1_000_000,
        }
    }
}

/// Why an import produced nothing at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportError {
    /// The reader failed. The message is the underlying error's own.
    Read { message: String },
    /// The input is larger than the byte budget.
    TooLarge { limit: u64 },
    /// The input holds more data rows than the row budget.
    TooManyRows { limit: usize },
    /// The input holds no record at all, so there is not even a header.
    NoHeader,
    /// A file that declares UTF-8 through its byte order mark and does not
    /// decode as it.
    InvalidUtf8,
    /// A file that declares UTF-16 through its byte order mark and then ends in
    /// the middle of a code unit.
    TruncatedUtf16,
    /// A file that declares UTF-16 and does not decode as it.
    InvalidUtf16,
    /// A column whose date order is still unanswered.
    ///
    /// This is the refusal that makes the question a question. The importer
    /// could pick an order and be right about half the files it ever sees; the
    /// half it is wrong about are wrong silently.
    UnansweredQuestion { column: usize, name: String },
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImportError::Read { message } => {
                write!(f, "the file could not be read: {message}")
            }
            ImportError::TooLarge { limit } => write!(
                f,
                "the file is larger than the {limit} byte budget for an import, \
                 so it was refused rather than read into memory"
            ),
            ImportError::TooManyRows { limit } => write!(
                f,
                "the file holds more than the {limit} row budget for an import, \
                 so it was refused rather than imported in part"
            ),
            ImportError::NoHeader => write!(
                f,
                "the file holds no record at all, so there is no header row to name the columns"
            ),
            ImportError::InvalidUtf8 => write!(
                f,
                "the file declares UTF-8 in its byte order mark and does not decode as UTF-8"
            ),
            ImportError::TruncatedUtf16 => write!(
                f,
                "the file declares UTF-16 in its byte order mark and ends in the \
                 middle of a code unit"
            ),
            ImportError::InvalidUtf16 => write!(
                f,
                "the file declares UTF-16 in its byte order mark and does not decode as UTF-16"
            ),
            ImportError::UnansweredQuestion { column, name } => write!(
                f,
                "column {column}, {name}, holds dates that are day-first under one \
                 reading and month-first under another. Answer that before importing; \
                 there is no order this importer could pick that would be right about \
                 both readings"
            ),
        }
    }
}

impl std::error::Error for ImportError {}

/// The text encoding a file turned out to be in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    /// UTF-8 with no byte order mark.
    Utf8,
    /// UTF-8 introduced by a byte order mark, which is what a spreadsheet on
    /// Windows writes.
    Utf8Bom,
    /// UTF-16, little endian, by its byte order mark.
    Utf16Le,
    /// UTF-16, big endian, by its byte order mark.
    Utf16Be,
    /// Not valid UTF-8 and carrying no byte order mark, so read as
    /// windows-1252, which is what such a file almost always is.
    Windows1252,
}

/// How the file separates its fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dialect {
    /// The field separator.
    pub delimiter: char,
    /// The encoding the bytes were decoded with.
    pub encoding: Encoding,
}

/// How the parts of a date are ordered in a column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateFormat {
    /// `YYYY-MM-DD`, which cannot be read any other way.
    Iso,
    /// `DD/MM/YYYY`.
    DayFirst,
    /// `MM/DD/YYYY`.
    MonthFirst,
}

/// What a column was judged to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnType {
    /// Anything that is not one of the others, which includes every column the
    /// importer declined to guess about.
    Text,
    /// Digits, one optional sign and one optional decimal point.
    Number,
    /// `true`, `false`, `yes` or `no`, in any case.
    Checkbox,
    /// A date. `None` means the order is still an open question and
    /// [`Source::import`] will refuse.
    Date(Option<DateFormat>),
}

/// Something the importer will not decide on the operator's behalf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Question {
    /// Every date in the column reads as a date under both orders. The examples
    /// are the values as they were written, so the operator is answering about
    /// their own file rather than about an abstraction.
    DateOrder { examples: Vec<String> },
}

/// One column, as the importer read it and as the operator may correct it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    /// The name from the header row, exactly as written.
    pub name: String,
    /// The type. Editing this is how a correction is made.
    pub kind: ColumnType,
    /// What the importer refused to decide, if anything.
    pub question: Option<Question>,
}

/// What will be imported, once the caller is content with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// One entry per column of the header row.
    pub columns: Vec<Column>,
}

/// What the importer found, before anything is imported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Survey {
    /// The delimiter and encoding that were detected.
    pub dialect: Dialect,
    /// The inferred plan, which the caller may edit and pass to
    /// [`Source::import`].
    pub plan: Plan,
    /// Data rows, not counting the header.
    pub rows: usize,
}

impl Survey {
    /// The columns the importer will not import without an answer.
    #[must_use]
    pub fn unanswered(&self) -> Vec<usize> {
        self.plan
            .columns
            .iter()
            .enumerate()
            .filter(|(_, c)| c.kind == ColumnType::Date(None))
            .map(|(i, _)| i)
            .collect()
    }
}

/// One cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// The field carried no bytes. Not zero, and not the empty string.
    Empty,
    /// Text, including a field written as `""`.
    Text(String),
    /// A number, carried as the lexeme that was validated. See the note at the
    /// top of the module about why it is not converted here.
    Number(String),
    /// A checkbox.
    Checkbox(bool),
    /// A date, as year, month and day.
    Date { year: i32, month: u32, day: u32 },
    /// A value that did not fit the column's type, kept exactly as written. A
    /// [`Flag`] names it.
    Raw(String),
}

/// Why a cell or a row was flagged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagReason {
    /// The value did not fit the column's type and was kept as written.
    NotOfColumnType,
    /// The row had fewer fields than the header, and was padded.
    ShortRow,
    /// The row had more fields than the header, and the extras were kept past
    /// the end of the row.
    LongRow,
}

/// Something the operator should see about one row, or one cell in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flag {
    /// The data row, counting from zero and not counting the header.
    pub row: usize,
    /// The column, where the flag is about one cell.
    pub column: Option<usize>,
    /// Why.
    pub reason: FlagReason,
    /// The value as it was written.
    pub value: String,
}

/// The imported table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    /// The columns, as they were imported under the plan that was applied.
    pub columns: Vec<Column>,
    /// One entry per data row. A row is as long as there are columns, unless
    /// the row carried more fields than that, in which case the extras are at
    /// the end as [`Value::Raw`].
    pub rows: Vec<Vec<Value>>,
    /// Everything that did not fit.
    pub flags: Vec<Flag>,
    /// What this import did to what it saw.
    ///
    /// It travels with the table rather than being printed and forgotten, so
    /// whatever stores the table can store the account of how it got there
    /// beside it. Where it says the same thing as [`Table::flags`] it says it
    /// counted, grouped and in the vocabulary every importer shares; the flags
    /// stay because they carry the value as written and the report deliberately
    /// carries only the position.
    pub report: Report,
}

/// One field, and whether it was written in quotes.
///
/// The flag is not a parsing detail. It is the whole of the difference between
/// a field that holds nothing and a field that holds an empty string, and the
/// two mean different things in the file.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Field {
    text: String,
    quoted: bool,
}

/// A decoded, parsed file, ready to survey and to import.
#[derive(Debug, Clone)]
pub struct Source {
    dialect: Dialect,
    header: Vec<Field>,
    records: Vec<Vec<Field>>,
}

/// How many records the delimiter detection looks at.
///
/// Bounded because detection is a heuristic and a heuristic that reads a
/// hundred megabytes to reach the same answer it reached in the first few
/// hundred rows is a slow heuristic rather than a better one.
const SNIFF_RECORDS: usize = 128;

/// The delimiters that are tried, in the order they are preferred on a tie.
///
/// Comma first because it names the format. Semicolon second because it is what
/// a spreadsheet writes wherever the comma is the decimal separator, which is
/// most of the places this product is aimed at.
const CANDIDATES: [char; 4] = [',', ';', '\t', '|'];

impl Source {
    /// Reads, decodes and parses, refusing anything past the budget.
    ///
    /// # Errors
    ///
    /// Returns [`ImportError::TooLarge`] or [`ImportError::TooManyRows`] when a
    /// budget is exceeded, [`ImportError::NoHeader`] for a file with no
    /// records, and the UTF-16 errors for a file whose byte order mark
    /// contradicts its contents.
    pub fn read(reader: &mut dyn Read, limits: &Limits) -> Result<Source, ImportError> {
        let mut bytes = Vec::new();
        // One byte past the budget, so that reaching the budget exactly and
        // exceeding it are distinguishable, and so that the refusal below has
        // read one byte more than the budget rather than the whole file.
        let mut bounded = reader.take(limits.max_bytes.saturating_add(1));
        bounded
            .read_to_end(&mut bytes)
            .map_err(|e| ImportError::Read {
                message: e.to_string(),
            })?;

        if bytes.len() as u64 > limits.max_bytes {
            return Err(ImportError::TooLarge {
                limit: limits.max_bytes,
            });
        }

        let (encoding, text) = decode(&bytes)?;
        let delimiter = detect_delimiter(&text);
        let mut records = parse(&text, delimiter, None);

        if records.is_empty() {
            return Err(ImportError::NoHeader);
        }

        let header = records.remove(0);

        if records.len() > limits.max_rows {
            return Err(ImportError::TooManyRows {
                limit: limits.max_rows,
            });
        }

        Ok(Source {
            dialect: Dialect {
                delimiter,
                encoding,
            },
            header,
            records,
        })
    }

    /// The fields one column actually holds a value in.
    ///
    /// An empty unquoted field is not evidence about anything, so it is left
    /// out here rather than in each caller. Inference and the declined-guess
    /// report have to agree about which fields they are looking at, and the way
    /// they agree is by asking the same question.
    fn present_in(&self, column: usize) -> Vec<&Field> {
        self.records
            .iter()
            .filter_map(|r| r.get(column))
            .filter(|f| !(f.text.is_empty() && !f.quoted))
            .collect()
    }

    /// What was detected, what each column looks like, and what is being asked.
    #[must_use]
    pub fn survey(&self) -> Survey {
        let columns = (0..self.header.len())
            .map(|i| {
                let name = self.header[i].text.clone();
                let present = self.present_in(i);

                let (kind, question) = infer(&present);
                Column {
                    name,
                    kind,
                    question,
                }
            })
            .collect();

        Survey {
            dialect: self.dialect,
            plan: Plan { columns },
            rows: self.records.len(),
        }
    }

    /// Applies a plan.
    ///
    /// # Errors
    ///
    /// Returns [`ImportError::UnansweredQuestion`] where the plan still carries
    /// a date column with no order. Nothing is imported in that case, because a
    /// half-imported table is the thing the operator has to undo by hand.
    pub fn import(&self, plan: &Plan) -> Result<Table, ImportError> {
        for (i, column) in plan.columns.iter().enumerate() {
            if column.kind == ColumnType::Date(None) {
                return Err(ImportError::UnansweredQuestion {
                    column: i,
                    name: column.name.clone(),
                });
            }
        }

        let width = plan.columns.len();
        let mut rows = Vec::with_capacity(self.records.len());
        let mut flags = Vec::new();

        for (r, record) in self.records.iter().enumerate() {
            if record.len() < width {
                flags.push(Flag {
                    row: r,
                    column: None,
                    reason: FlagReason::ShortRow,
                    value: format!("{} field(s) against {width} column(s)", record.len()),
                });
            } else if record.len() > width {
                flags.push(Flag {
                    row: r,
                    column: None,
                    reason: FlagReason::LongRow,
                    value: format!("{} field(s) against {width} column(s)", record.len()),
                });
            }

            let mut row = Vec::with_capacity(record.len().max(width));

            for c in 0..width {
                let Some(field) = record.get(c) else {
                    row.push(Value::Empty);
                    continue;
                };

                let value = convert(field, plan.columns[c].kind);
                if matches!(value, Value::Raw(_)) {
                    flags.push(Flag {
                        row: r,
                        column: Some(c),
                        reason: FlagReason::NotOfColumnType,
                        value: field.text.clone(),
                    });
                }
                row.push(value);
            }

            // Past the header's width. Kept rather than dropped: the file said
            // something here and the operator is the one who gets to decide it
            // was noise.
            for field in record.iter().skip(width) {
                row.push(Value::Raw(field.text.clone()));
            }

            rows.push(row);
        }

        let report = self.report_on(plan, &rows, &flags);

        Ok(Table {
            columns: plan.columns.clone(),
            rows,
            flags,
            report,
        })
    }

    /// The account of one import, built from what it produced.
    ///
    /// Derived rather than accumulated as the import runs, so there is no way
    /// for the table and the report to describe two different imports. The cost
    /// is one more pass over the flags, which is bounded by the rows that were
    /// already read.
    fn report_on(&self, plan: &Plan, rows: &[Vec<Value>], flags: &[Flag]) -> Report {
        let mut report = Report::new(
            "csv",
            Counts {
                columns: plan.columns.len(),
                rows: rows.len(),
                cells: rows.iter().map(Vec::len).sum(),
            },
        );

        // What a delimited file cannot carry at all. Constant for this format
        // rather than read off this file, and that is the honest form: the
        // question a person arrives with is what happened to their formulas,
        // and the answer is about the format they exported through rather than
        // about their data.
        report.add(Entry::whole(
            State::NotInSource,
            "formula",
            "a delimited file carries the computed value of a cell and never the \
             expression behind it, so no formula reached this importer",
        ));
        report.add(Entry::whole(
            State::NotInSource,
            "cell formatting",
            "colour, font, width and number formatting are not written to a \
             delimited file",
        ));

        // A guess this importer declines to make, per column. The column came
        // in as text either way; naming the construct is the difference between
        // a person knowing why and having to work it out from the data.
        for (c, column) in plan.columns.iter().enumerate() {
            if column.kind != ColumnType::Text {
                continue;
            }
            let Some((construct, detail)) = declined(&self.present_in(c)) else {
                continue;
            };
            report.add(Entry::at(
                State::NotSupported,
                construct,
                detail,
                vec![format!("column {c}, {}", column.name)],
            ));
        }

        for (reason, construct, detail) in [
            (
                FlagReason::NotOfColumnType,
                "value outside its column's type",
                "kept exactly as written, as an unparsed cell, rather than \
                 coerced or dropped",
            ),
            (
                FlagReason::ShortRow,
                "row shorter than the header",
                "padded to the width of the columns with cells that carry \
                 nothing, which is not the same as cells that carry an empty \
                 string",
            ),
            (
                FlagReason::LongRow,
                "row longer than the header",
                "the fields past the last column were kept at the end of the \
                 row, outside any column, rather than discarded",
            ),
        ] {
            let sites: Vec<String> = flags
                .iter()
                .filter(|f| f.reason == reason)
                .map(|f| match f.column {
                    Some(c) => format!("row {}, column {c}", f.row),
                    None => format!("row {}", f.row),
                })
                .collect();
            report.add(Entry::at(State::Degraded, construct, detail, sites));
        }

        report
    }
}

/// A guess this importer refuses to make about a column that came in as text.
///
/// Only the two the module documents as deliberate refusals. A column that is
/// text because it holds text is not a finding, and reporting it would bury the
/// two cases that are.
///
/// Both tests are unanimous over the fields that carry a value: one odd value
/// in a column of grouped numbers means the column is not a column of grouped
/// numbers, and claiming otherwise would put a construct in the report that the
/// file does not have.
fn declined(present: &[&Field]) -> Option<(&'static str, &'static str)> {
    if present.is_empty() {
        return None;
    }

    let grouped = present
        .iter()
        .any(|f| is_grouped_number(&f.text) && !is_number(&f.text));
    if grouped
        && present
            .iter()
            .all(|f| is_number(&f.text) || is_grouped_number(&f.text))
    {
        return Some((
            "thousands separator",
            "the column reads as numbers written with a group separator, which is \
             locale-dependent and is not in the file, so it was imported as text \
             with every character intact",
        ));
    }

    let two_digit = present.iter().any(|f| is_two_digit_year_date(&f.text));
    if two_digit
        && present
            .iter()
            .all(|f| is_two_digit_year_date(&f.text) || date_shape(&f.text).is_some())
    {
        return Some((
            "two-digit year",
            "the column reads as dates whose year is two digits, and the century \
             would have to come from a convention rather than from the file, so it \
             was imported as text with every character intact",
        ));
    }

    None
}

/// The characters a thousands separator is written with.
///
/// The space is the ASCII one. A file using a non-breaking space is not read as
/// grouped here, and that is a gap rather than a decision: it means such a
/// column is reported as nothing rather than as this construct.
const GROUPING: [char; 3] = [',', '.', ' '];

/// A number written with a group separator, which [`is_number`] refuses.
///
/// One character does the grouping and a different one may then mark the
/// decimal, because no notation uses one character for both. Every group after
/// the first is exactly three digits, which is what makes this a separator
/// rather than a decimal point with too many digits after it.
fn is_grouped_number(text: &str) -> bool {
    let body = text.strip_prefix('-').unwrap_or(text);
    let body = body.strip_prefix('+').unwrap_or(body);
    if body.is_empty() {
        return false;
    }

    for group in GROUPING {
        for point in GROUPING.into_iter().filter(|p| *p != group) {
            let (whole, fraction) = match body.rsplit_once(point) {
                Some((whole, fraction)) => (whole, Some(fraction)),
                None => (body, None),
            };

            // No let-chain: the workspace floor in `rust-version` predates
            // them, and the `msrv` job in the gate builds with it.
            if let Some(fraction) = fraction {
                if fraction.is_empty() || !fraction.bytes().all(|b| b.is_ascii_digit()) {
                    continue;
                }
            }

            let parts: Vec<&str> = whole.split(group).collect();
            if parts.len() < 2 {
                continue;
            }
            if !parts
                .iter()
                .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
            {
                continue;
            }
            if parts[0].len() > 3 {
                continue;
            }
            if parts[1..].iter().all(|p| p.len() == 3) {
                return true;
            }
        }
    }

    false
}

/// A date whose year is two digits, which [`date_shape`] refuses.
///
/// The same three parts and the same separators, and the last part is two
/// digits rather than four. Recognising it here is not the importer changing
/// its mind about reading it; it is the importer being able to say what it
/// declined.
fn is_two_digit_year_date(text: &str) -> bool {
    let Some(separator) = ['-', '/', '.'].into_iter().find(|s| text.contains(*s)) else {
        return false;
    };
    let parts: Vec<&str> = text.split(separator).collect();
    if parts.len() != 3 {
        return false;
    }
    if !parts
        .iter()
        .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
    {
        return false;
    }

    parts[0].len() <= 2 && parts[1].len() <= 2 && parts[2].len() == 2
}

/// Turns a field into a value under a column's type.
///
/// A field that does not fit becomes [`Value::Raw`] rather than a default, a
/// zero or an absence. Every one of those would be the importer inventing a
/// value the file did not contain.
fn convert(field: &Field, kind: ColumnType) -> Value {
    if field.text.is_empty() && !field.quoted {
        return Value::Empty;
    }

    match kind {
        ColumnType::Text => Value::Text(field.text.clone()),
        ColumnType::Number => {
            if is_number(&field.text) {
                Value::Number(field.text.clone())
            } else {
                Value::Raw(field.text.clone())
            }
        }
        ColumnType::Checkbox => match as_checkbox(&field.text) {
            Some(b) => Value::Checkbox(b),
            None => Value::Raw(field.text.clone()),
        },
        ColumnType::Date(Some(format)) => match as_date(&field.text, format) {
            Some((year, month, day)) => Value::Date { year, month, day },
            None => Value::Raw(field.text.clone()),
        },
        // Refused in `import` before any row is converted, so this is only
        // reachable by a caller that built a `Table` some other way.
        ColumnType::Date(None) => Value::Raw(field.text.clone()),
    }
}

/// Judges one column from the values that are actually present in it.
///
/// Empty fields are not evidence: a column of numbers with gaps is still a
/// column of numbers, and treating a gap as a value would make every such
/// column text.
fn infer(present: &[&Field]) -> (ColumnType, Option<Question>) {
    if present.is_empty() {
        return (ColumnType::Text, None);
    }

    if present.iter().all(|f| as_checkbox(&f.text).is_some()) {
        return (ColumnType::Checkbox, None);
    }

    if present.iter().all(|f| is_number(&f.text)) {
        return (ColumnType::Number, None);
    }

    infer_date(present)
}

/// The shape of one date-looking value, before any order is chosen.
enum Shape {
    /// Four-digit year first, which is unambiguous.
    Iso(i32, u32, u32),
    /// Two one-or-two-digit parts and a four-digit year, in that order.
    Parts(u32, u32, i32),
}

/// Judges a column of dates, and asks rather than guesses where it must.
fn infer_date(present: &[&Field]) -> (ColumnType, Option<Question>) {
    let mut shapes = Vec::with_capacity(present.len());
    for field in present {
        match date_shape(&field.text) {
            Some(shape) => shapes.push(shape),
            None => return (ColumnType::Text, None),
        }
    }

    let iso = shapes
        .iter()
        .filter(|s| matches!(s, Shape::Iso(..)))
        .count();
    if iso == shapes.len() {
        // Validate, because `2024-02-31` has the shape of a date and is not one.
        let all_real = shapes.iter().all(|s| match s {
            Shape::Iso(y, m, d) => is_real_date(*y, *m, *d),
            Shape::Parts(..) => false,
        });
        return if all_real {
            (ColumnType::Date(Some(DateFormat::Iso)), None)
        } else {
            (ColumnType::Text, None)
        };
    }

    if iso != 0 {
        // A column carrying both shapes is two columns that were concatenated,
        // or a file whose writer changed its mind. Either way it is not one
        // date column, and text is what keeps every byte.
        return (ColumnType::Text, None);
    }

    let mut first_over_twelve = false;
    let mut second_over_twelve = false;
    for shape in &shapes {
        if let Shape::Parts(a, b, _) = shape {
            first_over_twelve |= *a > 12;
            second_over_twelve |= *b > 12;
        }
    }

    // Both, so the column contradicts itself: some rows can only be day-first
    // and others can only be month-first. There is no order under which the
    // whole column reads, and inventing one would silently move dates.
    if first_over_twelve && second_over_twelve {
        return (ColumnType::Text, None);
    }

    let format = if first_over_twelve {
        Some(DateFormat::DayFirst)
    } else if second_over_twelve {
        Some(DateFormat::MonthFirst)
    } else {
        None
    };

    let real_under = |format: DateFormat| {
        shapes.iter().all(|s| match s {
            Shape::Parts(a, b, y) => match format {
                DateFormat::DayFirst => is_real_date(*y, *b, *a),
                DateFormat::MonthFirst => is_real_date(*y, *a, *b),
                DateFormat::Iso => false,
            },
            Shape::Iso(..) => false,
        })
    };

    match format {
        Some(format) => {
            if real_under(format) {
                (ColumnType::Date(Some(format)), None)
            } else {
                (ColumnType::Text, None)
            }
        }
        None => {
            // Every part is twelve or below, so both readings are complete. If
            // one of them produces a date that does not exist the ambiguity is
            // resolved by arithmetic rather than by a question; where both hold,
            // the question is the only honest answer.
            let day_first = real_under(DateFormat::DayFirst);
            let month_first = real_under(DateFormat::MonthFirst);
            match (day_first, month_first) {
                (true, true) => {
                    let examples: Vec<String> = present
                        .iter()
                        .take(3)
                        .map(|f| f.text.clone())
                        .collect::<Vec<String>>();
                    (
                        ColumnType::Date(None),
                        Some(Question::DateOrder { examples }),
                    )
                }
                (true, false) => (ColumnType::Date(Some(DateFormat::DayFirst)), None),
                (false, true) => (ColumnType::Date(Some(DateFormat::MonthFirst)), None),
                (false, false) => (ColumnType::Text, None),
            }
        }
    }
}

/// `true`, `false`, `yes` or `no`, in any case.
///
/// `1` and `0` are deliberately absent. They are numbers, and a column of them
/// is a column of numbers far more often than it is a column of checkboxes.
fn as_checkbox(text: &str) -> Option<bool> {
    match text.to_ascii_lowercase().as_str() {
        "true" | "yes" => Some(true),
        "false" | "no" => Some(false),
        _ => None,
    }
}

/// An optional sign, digits, and at most one decimal point with digits after it.
///
/// No thousands separator, no exponent, no currency symbol and no spaces. Each
/// of those needs a locale to read, and the locale is not in the file.
fn is_number(text: &str) -> bool {
    let body = text.strip_prefix('-').unwrap_or(text);
    let body = body.strip_prefix('+').unwrap_or(body);

    let mut parts = body.split('.');
    let whole = parts.next().unwrap_or("");
    let fraction = parts.next();
    if parts.next().is_some() {
        return false;
    }

    if whole.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }

    match fraction {
        None => true,
        Some(f) => !f.is_empty() && f.bytes().all(|b| b.is_ascii_digit()),
    }
}

/// The shape of a date, without choosing an order.
fn date_shape(text: &str) -> Option<Shape> {
    let separator = ['-', '/', '.'].into_iter().find(|s| text.contains(*s))?;
    let parts: Vec<&str> = text.split(separator).collect();
    if parts.len() != 3 {
        return None;
    }
    if !parts
        .iter()
        .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
    {
        return None;
    }

    if parts[0].len() == 4 {
        let year: i32 = parts[0].parse().ok()?;
        let month: u32 = parts[1].parse().ok()?;
        let day: u32 = parts[2].parse().ok()?;
        if parts[1].len() > 2 || parts[2].len() > 2 {
            return None;
        }
        return Some(Shape::Iso(year, month, day));
    }

    // A two-digit year would need a century supplied by convention rather than
    // read from the file, so it is not a date here.
    if parts[2].len() != 4 || parts[0].len() > 2 || parts[1].len() > 2 {
        return None;
    }

    let a: u32 = parts[0].parse().ok()?;
    let b: u32 = parts[1].parse().ok()?;
    let year: i32 = parts[2].parse().ok()?;
    if a == 0 || b == 0 || a > 31 || b > 31 {
        return None;
    }

    Some(Shape::Parts(a, b, year))
}

/// Reads one value under a chosen order.
fn as_date(text: &str, format: DateFormat) -> Option<(i32, u32, u32)> {
    match (date_shape(text)?, format) {
        (Shape::Iso(y, m, d), DateFormat::Iso) if is_real_date(y, m, d) => Some((y, m, d)),
        (Shape::Parts(a, b, y), DateFormat::DayFirst) if is_real_date(y, b, a) => Some((y, b, a)),
        (Shape::Parts(a, b, y), DateFormat::MonthFirst) if is_real_date(y, a, b) => Some((y, a, b)),
        _ => None,
    }
}

/// Whether a year, month and day name a day that exists.
fn is_real_date(year: i32, month: u32, day: u32) -> bool {
    if !(1..=12).contains(&month) || day == 0 {
        return false;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let length = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    day <= length
}

/// Decodes the bytes, reporting which encoding they turned out to be.
///
/// # Errors
///
/// A byte order mark is a declaration, so a file that carries one and then does
/// not decode is refused rather than read some other way. Guessing past an
/// explicit statement in the file is how an importer produces a table nobody
/// can explain.
fn decode(bytes: &[u8]) -> Result<(Encoding, String), ImportError> {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        let text = std::str::from_utf8(rest).map_err(|_| ImportError::InvalidUtf8)?;
        return Ok((Encoding::Utf8Bom, text.to_owned()));
    }

    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        return Ok((Encoding::Utf16Le, decode_utf16(rest, u16::from_le_bytes)?));
    }

    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        return Ok((Encoding::Utf16Be, decode_utf16(rest, u16::from_be_bytes)?));
    }

    Ok(decode_utf8_or_1252(bytes))
}

/// UTF-8 where the bytes are valid UTF-8, and windows-1252 where they are not.
///
/// The fallback is not a guess about the world in general. A file that is not
/// valid UTF-8 and carries no mark is, in practice, a spreadsheet export from a
/// Windows machine, and windows-1252 reads every byte rather than refusing
/// some, so nothing is lost that could have been kept.
fn decode_utf8_or_1252(bytes: &[u8]) -> (Encoding, String) {
    match std::str::from_utf8(bytes) {
        Ok(text) => (Encoding::Utf8, text.to_owned()),
        Err(_) => (
            Encoding::Windows1252,
            bytes.iter().map(|b| windows_1252_char(*b)).collect(),
        ),
    }
}

/// One windows-1252 byte, as its character.
///
/// The bytes outside `0x80..=0x9F` are Latin-1 and map to the same code point.
/// The five positions the encoding leaves undefined map to the C1 control at
/// the same code point, which is what the WHATWG encoding standard specifies
/// and is what a reader comparing against a browser will expect.
fn windows_1252_char(byte: u8) -> char {
    const HIGH: [char; 32] = [
        '\u{20AC}', '\u{0081}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}',
        '\u{2021}', '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{008D}',
        '\u{017D}', '\u{008F}', '\u{0090}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}',
        '\u{2022}', '\u{2013}', '\u{2014}', '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}',
        '\u{0153}', '\u{009D}', '\u{017E}', '\u{0178}',
    ];

    if (0x80..=0x9F).contains(&byte) {
        HIGH[usize::from(byte - 0x80)]
    } else {
        char::from(byte)
    }
}

/// Decodes UTF-16 in the endianness the byte order mark declared.
fn decode_utf16(bytes: &[u8], unit: fn([u8; 2]) -> u16) -> Result<String, ImportError> {
    if bytes.len() % 2 != 0 {
        return Err(ImportError::TruncatedUtf16);
    }
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| unit([c[0], c[1]]))
        .collect::<Vec<u16>>();
    String::from_utf16(&units).map_err(|_| ImportError::InvalidUtf16)
}

/// Picks the delimiter, from a bounded window at the start of the file.
///
/// The rule is the one a person uses: the separator that makes the file
/// rectangular. For each candidate the window is parsed, the most common field
/// count is taken, and the candidate wins that produces the widest such count,
/// with the number of records agreeing on that width settling a tie. Ties past
/// that go to the earlier candidate.
///
/// Its bound is stated rather than hidden. A file with one column has no
/// delimiter to find, and nothing could find one; it falls back to a comma and
/// reads as one column, which is the right answer for it. A file whose first
/// window is rectangular under the wrong candidate is read under the wrong
/// candidate, and the operator sees that in the survey before anything is
/// imported, which is what the survey is for.
fn detect_delimiter(text: &str) -> char {
    let mut best = (CANDIDATES[0], 0usize, 0usize);

    for candidate in CANDIDATES {
        let records = parse(text, candidate, Some(SNIFF_RECORDS));
        if records.is_empty() {
            continue;
        }

        let mut counts: Vec<(usize, usize)> = Vec::new();
        for record in &records {
            match counts.iter_mut().find(|(width, _)| *width == record.len()) {
                Some((_, seen)) => *seen += 1,
                None => counts.push((record.len(), 1)),
            }
        }
        let (modal, at_modal) = counts
            .iter()
            .max_by_key(|(width, seen)| (*seen, *width))
            .map_or((0, 0), |(width, seen)| (*width, *seen));

        // Width first, then how many records agree on it. The second term is
        // what separates a real separator from a character that merely occurs
        // in the data: a semicolon-separated file whose numbers carry comma
        // thousands separators is two fields wide under both candidates, and
        // only the semicolon makes every record two fields wide.
        if (modal, at_modal) > (best.1, best.2) {
            best = (candidate, modal, at_modal);
        }
    }

    best.0
}

/// Splits text into records and fields.
///
/// Quoting is RFC 4180's: a field that opens with a quote runs to the next
/// unpaired quote, a doubled quote inside one is a literal quote, and a
/// delimiter or a line ending inside one is data rather than structure. Line
/// endings are `\r\n`, `\n` or a bare `\r`, because all three are in files that
/// exist.
///
/// A record consisting of exactly one unquoted empty field is a blank line and
/// is dropped. That is the only thing this parser discards, and it is the one
/// case where keeping it would produce a row of empty cells for every blank
/// line in the file.
fn parse(text: &str, delimiter: char, max_records: Option<usize>) -> Vec<Vec<Field>> {
    let mut records: Vec<Vec<Field>> = Vec::new();
    let mut record: Vec<Field> = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut in_quotes = false;

    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if in_quotes {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    in_quotes = false;
                }
            } else {
                field.push(c);
            }
            continue;
        }

        if c == '"' && field.is_empty() && !quoted {
            in_quotes = true;
            quoted = true;
            continue;
        }

        if c == delimiter {
            record.push(Field {
                text: std::mem::take(&mut field),
                quoted,
            });
            quoted = false;
            continue;
        }

        if c == '\n' || c == '\r' {
            if c == '\r' && chars.peek() == Some(&'\n') {
                chars.next();
            }
            record.push(Field {
                text: std::mem::take(&mut field),
                quoted,
            });
            quoted = false;
            push_record(&mut records, std::mem::take(&mut record));
            if let Some(max) = max_records {
                if records.len() >= max {
                    return records;
                }
            }
            continue;
        }

        field.push(c);
    }

    if !field.is_empty() || quoted || !record.is_empty() {
        record.push(Field {
            text: field,
            quoted,
        });
        push_record(&mut records, record);
    }

    records
}

/// Adds a record unless it is a blank line.
fn push_record(records: &mut Vec<Vec<Field>>, record: Vec<Field>) {
    let blank = record.len() == 1 && record[0].text.is_empty() && !record[0].quoted;
    if !blank {
        records.push(record);
    }
}
