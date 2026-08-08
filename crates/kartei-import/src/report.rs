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

//! What an import did to what it saw, produced by the run rather than written
//! down beside it.
//!
//! A fidelity note in the documentation says what an importer dropped when
//! somebody last looked. The importer then changes and the note does not, and a
//! reader who trusts it is worse off than one who had nothing. This module is
//! the other shape: every import produces its own account, so the account is
//! about the file that was actually imported and cannot describe a version of
//! the importer that no longer exists.
//!
//! # Three states, and they are not interchangeable
//!
//! [`State`] separates the three answers a person is owed when they ask what
//! happened to their formulas, and the separation is the point of the type.
//! "The export never carried it", "the export carried it and this importer does
//! not read it" and "it was imported and something about it was lost" are three
//! different facts with three different repairs. A report that collapses any
//! two of them into "skipped" is a report that cannot be acted on: the first is
//! answered by exporting differently, the second by an issue on this tracker,
//! and the third by looking at the affected rows.
//!
//! # An import that skipped nothing says so
//!
//! [`Report::nothing_was_skipped`] is a question the caller can ask, and the
//! rendered form ends with the answer in words. Without that, a report with no
//! entries reads exactly like a report from an importer that stopped looking,
//! which is the failure this whole module exists against.
//!
//! # Bounds
//!
//! An entry counts every occurrence and lists up to [`SITES_SHOWN`] of them.
//! The count is the truth and the list is a way in, so a million bad cells
//! produce a report a person can read rather than a second copy of the file.
//! Where the list is shorter than the count, the rendered form says how many
//! were not listed rather than trailing off.

use std::fmt;

/// How many occurrences an entry names before it starts counting silently.
///
/// Enough to find a pattern by eye, few enough that the report stays a report.
/// The count beside them is never bounded, so a truncated list is still an
/// exact number.
pub const SITES_SHOWN: usize = 8;

/// What happened to one construct.
///
/// The three are exhaustive over what an importer can do with something it saw
/// or did not see, and they are deliberately not orderable: none of them is a
/// worse version of another.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// It was not in what the importer read.
    ///
    /// This is a statement about the export rather than about the importer, and
    /// it is the answer that sends somebody back to the tool they exported
    /// from.
    NotInSource,
    /// It was there, and this importer does not read it.
    ///
    /// The bytes were seen and were not understood. Whatever the importer did
    /// with them instead is in the entry's detail.
    NotSupported,
    /// It was imported, and something about it was lost.
    ///
    /// The data is in the workspace and is not what the file said. This is the
    /// state that needs the affected rows named, because it is the only one
    /// where there is something to go and look at.
    Degraded,
}

impl State {
    /// The heading this state is reported under.
    #[must_use]
    pub fn heading(self) -> &'static str {
        match self {
            State::NotInSource => "not present in the source",
            State::NotSupported => "present in the source and not supported by this importer",
            State::Degraded => "imported with a stated loss",
        }
    }
}

/// One construct, what happened to it, and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Which of the three.
    pub state: State,
    /// The construct, named the way somebody looking for it would name it.
    pub construct: String,
    /// What happened, in a sentence, including what was kept where anything
    /// was.
    pub detail: String,
    /// How many times, unbounded and exact.
    pub count: usize,
    /// Where, up to [`SITES_SHOWN`] of them. Shorter than `count` is normal and
    /// is disclosed when the entry is rendered.
    pub sites: Vec<String>,
}

impl Entry {
    /// An entry with no site list, for a construct that is about the whole
    /// import rather than about particular rows.
    #[must_use]
    pub fn whole(state: State, construct: &str, detail: &str) -> Entry {
        Entry {
            state,
            construct: construct.to_owned(),
            detail: detail.to_owned(),
            count: 1,
            sites: Vec::new(),
        }
    }

    /// An entry counting `count` occurrences and naming the first few.
    ///
    /// The caller passes every site it has; the truncation happens here so that
    /// no caller has to remember to do it.
    #[must_use]
    pub fn at(state: State, construct: &str, detail: &str, sites: Vec<String>) -> Entry {
        let count = sites.len();
        let mut sites = sites;
        sites.truncate(SITES_SHOWN);
        Entry {
            state,
            construct: construct.to_owned(),
            detail: detail.to_owned(),
            count,
            sites,
        }
    }

    /// How many occurrences this entry counted and did not name.
    #[must_use]
    pub fn unnamed(&self) -> usize {
        self.count.saturating_sub(self.sites.len())
    }
}

/// What arrived, in numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Counts {
    /// Columns in the imported table.
    pub columns: usize,
    /// Data rows, not counting a header.
    pub rows: usize,
    /// Cells in the imported table, counting empty ones and anything kept past
    /// the width the columns declare.
    pub cells: usize,
}

/// What one import did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// Which importer produced this.
    pub importer: String,
    /// What arrived.
    pub counts: Counts,
    /// What happened to everything else.
    pub entries: Vec<Entry>,
}

impl Report {
    /// An empty report from a named importer.
    #[must_use]
    pub fn new(importer: &str, counts: Counts) -> Report {
        Report {
            importer: importer.to_owned(),
            counts,
            entries: Vec::new(),
        }
    }

    /// Adds an entry, dropping one that counted nothing.
    ///
    /// A zero-count entry is a construct that did not occur, and reporting it
    /// beside the ones that did is how a report becomes something nobody reads.
    pub fn add(&mut self, entry: Entry) {
        if entry.count == 0 {
            return;
        }
        self.entries.push(entry);
    }

    /// The entries in one state, in the order they were added.
    pub fn in_state(&self, state: State) -> impl Iterator<Item = &Entry> {
        self.entries.iter().filter(move |e| e.state == state)
    }

    /// Whether any construct was named as unsupported or as degraded.
    ///
    /// [`State::NotInSource`] does not count against it. Something the export
    /// never carried was not skipped by this importer, and folding the two
    /// together would make every import look lossy.
    #[must_use]
    pub fn nothing_was_skipped(&self) -> bool {
        !self
            .entries
            .iter()
            .any(|e| matches!(e.state, State::NotSupported | State::Degraded))
    }

    /// Whether a construct is named anywhere in the report.
    ///
    /// Exact rather than substring, because a report is searched for a name
    /// somebody was given rather than for a fragment they guessed.
    #[must_use]
    pub fn names(&self, construct: &str) -> bool {
        self.entries.iter().any(|e| e.construct == construct)
    }
}

impl fmt::Display for Report {
    /// The report as a person reads it.
    ///
    /// Every state gets its heading even when it holds nothing, so a reader
    /// learns that the importer looked and found none rather than that it did
    /// not look. The last line is the explicit answer about skipping.
    ///
    /// An entry with no sites is a fact about the whole import and is printed
    /// without a count, because counting it once would read as one occurrence
    /// of something that has none. That case is exactly [`Entry::whole`]:
    /// [`Entry::at`] derives its count from its sites and [`Report::add`] drops
    /// an entry that counted nothing, so an entry with a count worth printing
    /// always has at least one site.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} import", self.importer)?;
        writeln!(
            f,
            "{} column(s), {} row(s) and {} cell(s) imported.",
            self.counts.columns, self.counts.rows, self.counts.cells
        )?;

        for state in [State::NotInSource, State::NotSupported, State::Degraded] {
            writeln!(f)?;
            writeln!(f, "{}", state.heading())?;

            let mut any = false;
            for entry in self.in_state(state) {
                any = true;
                if entry.sites.is_empty() {
                    writeln!(f, "  {}: {}", entry.construct, entry.detail)?;
                    continue;
                }
                writeln!(
                    f,
                    "  {} ({}): {}",
                    entry.construct, entry.count, entry.detail
                )?;
                for site in &entry.sites {
                    writeln!(f, "    {site}")?;
                }
                let unnamed = entry.unnamed();
                if unnamed != 0 {
                    writeln!(f, "    and {unnamed} more, not listed")?;
                }
            }
            if !any {
                writeln!(f, "  none")?;
            }
        }

        writeln!(f)?;
        if self.nothing_was_skipped() {
            write!(
                f,
                "Nothing was skipped and nothing was carried in a degraded form."
            )
        } else {
            write!(
                f,
                "Something was skipped or degraded. The entries above are the whole of it."
            )
        }
    }
}
