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

//! One configuration file, environment overrides, and an unknown key refused.
//!
//! The failure this exists against is a misspelled key that silently means the
//! default. A security setting typed `requrie_tls` is off, the file looks like
//! it says otherwise, and nothing anywhere reports the difference. So the set of
//! keys is closed: a key this file does not know is an error at startup and not
//! a warning, and the message names the key and the nearest key that is known,
//! because an operator who typed one character wrong should not have to read a
//! document to find out which one.
//!
//! The key set is enumerated here rather than derived from a deserialiser's
//! `deny_unknown_fields` switch, and the distance between two key names is
//! computed here too. Both are rules this repository owns. Taking them from a
//! library would make the product's fail-closed behaviour a property of whatever
//! parser happened to be chosen, and it would move with that parser's next major
//! version.
//!
//! `SCHEMA` is deliberately short. Every key in it is one the binary itself owns
//! today. A subsystem that lands later brings its own keys, and it brings them by
//! adding to this list rather than by widening what an unknown key means, which
//! is what keeps the refusal above worth anything.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use toml::de::{DeTable, DeValue};

/// Where a key may be set from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Settable {
    /// The configuration file, or the environment variable that overrides it.
    FileOrEnvironment,
    /// The environment only. This is what a secret carries: the file may name a
    /// path to read a secret out of, and never the secret itself, so a
    /// configuration file can be shown to somebody without leaking.
    EnvironmentOnly,
}

/// Where the value in force actually came from.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Origin {
    /// Nothing set it; the value is the one in `SCHEMA`.
    Default,
    /// The configuration file.
    File,
    /// The environment variable for the key.
    Environment,
    /// Read out of the file named by another key.
    ReadFrom(String),
    /// No value, and the key has no default.
    Unset,
}

impl Origin {
    fn label(&self) -> String {
        match self {
            Origin::Default => "default".to_owned(),
            Origin::File => "configuration file".to_owned(),
            Origin::Environment => "environment".to_owned(),
            Origin::ReadFrom(path) => format!("read from {path}"),
            Origin::Unset => "unset".to_owned(),
        }
    }
}

/// One key, with everything about it in one place: what it is for, what it
/// defaults to, where it may be set from, whether its value is a secret, and
/// what makes a value invalid.
pub struct Key {
    /// The dotted path, as it is written in the file.
    pub path: &'static str,
    /// What the key is for, one sentence per line. This is what
    /// `print-config` emits as comments above the key, which is how the
    /// requirement that every key is documented in the file itself is met
    /// without a second copy of the documentation to drift against this one.
    pub doc: &'static [&'static str],
    /// The value in force when nothing sets it, or `None` where there cannot
    /// be one.
    pub default: Option<&'static str>,
    pub settable: Settable,
    /// A value that must never be printed.
    pub secret: bool,
    /// What the key will not accept. Returns the reason, so the message an
    /// operator sees is written next to the rule rather than assembled at the
    /// call site.
    pub check: fn(&str) -> Result<(), String>,
}

impl Key {
    /// The environment variable that overrides this key.
    ///
    /// Derived rather than written out, so a key cannot arrive with a variable
    /// name that does not follow the rule. The derivation is not injective on
    /// its own - `data.directory` and `data_directory` would both produce
    /// `KARTEI_DATA_DIRECTORY` - so `the_schema_gives_every_key_its_own_variable`
    /// in the suite refuses a schema where two keys collide.
    pub fn variable(&self) -> String {
        let mut name = String::from("KARTEI_");
        for c in self.path.chars() {
            name.push(match c {
                '.' | '-' => '_',
                other => other.to_ascii_uppercase(),
            });
        }
        name
    }
}

fn accept_anything(_: &str) -> Result<(), String> {
    Ok(())
}

fn is_a_socket_address(value: &str) -> Result<(), String> {
    match value.parse::<std::net::SocketAddr>() {
        Ok(_) => Ok(()),
        Err(_) => Err(format!(
            "{value:?} is not an address and a port. Write it as 127.0.0.1:8080, \
             or as [::1]:8080 for IPv6."
        )),
    }
}

/// The levels, most severe first, so the message lists them in the order a
/// reader expects rather than alphabetically.
const LOG_LEVELS: [&str; 5] = ["error", "warn", "info", "debug", "trace"];

fn is_a_log_level(value: &str) -> Result<(), String> {
    if LOG_LEVELS.contains(&value) {
        Ok(())
    } else {
        Err(format!(
            "{value:?} is not a level. The levels are {}.",
            LOG_LEVELS.join(", ")
        ))
    }
}

/// Every key this server knows.
pub const SCHEMA: &[Key] = &[
    Key {
        path: "listen.address",
        doc: &[
            "The address and port the server binds.",
            "The default is the loopback interface, so a first start is not reachable",
            "from the network until an operator decides it should be. Put a reverse",
            "proxy in front of it or set 0.0.0.0:8080 to bind every interface.",
        ],
        default: Some("127.0.0.1:8080"),
        settable: Settable::FileOrEnvironment,
        secret: false,
        check: is_a_socket_address,
    },
    Key {
        path: "data.directory",
        doc: &[
            "The one directory everything this server keeps is kept under.",
            "Relative paths are resolved against the working directory the server",
            "was started in, which is not the directory this file is in.",
        ],
        default: Some("./data"),
        settable: Settable::FileOrEnvironment,
        secret: false,
        check: accept_anything,
    },
    Key {
        path: "log.level",
        doc: &["How much the server writes about what it is doing."],
        default: Some("info"),
        settable: Settable::FileOrEnvironment,
        secret: false,
        check: is_a_log_level,
    },
    Key {
        path: "admin.token_file",
        doc: &[
            "A file holding the first administrator's token, one line.",
            "The path is not a secret and belongs here; the token is a secret and",
            "does not. Leave this unset and set KARTEI_ADMIN_TOKEN instead where a",
            "container's secret arrives as an environment variable.",
        ],
        default: None,
        settable: Settable::FileOrEnvironment,
        secret: false,
        check: accept_anything,
    },
    Key {
        path: "admin.token",
        doc: &[
            "The first administrator's token itself.",
            "This key is refused in the file. Set KARTEI_ADMIN_TOKEN, or point",
            "admin.token_file at a file holding it.",
        ],
        default: None,
        settable: Settable::EnvironmentOnly,
        secret: true,
        check: accept_anything,
    },
];

fn key(path: &str) -> Option<&'static Key> {
    SCHEMA.iter().find(|k| k.path == path)
}

/// One thing wrong with the configuration.
///
/// Loading collects every problem rather than stopping at the first, because an
/// operator fixing a file one restart at a time is an operator who stops reading
/// the output.
#[derive(Debug, PartialEq, Eq)]
pub struct Problem {
    pub message: String,
}

/// The value in force for one key, and where it came from.
#[derive(Debug, Clone)]
pub struct Setting {
    pub path: &'static str,
    pub value: Option<String>,
    pub origin: Origin,
    pub secret: bool,
}

/// The configuration in force.
///
/// There is no accessor for a single key here yet, and that is the state rather
/// than an oversight. Nothing in this binary reads a value: no socket is bound,
/// no directory is opened and nothing is logged, so an accessor would be a
/// function with no caller, which the build refuses. It arrives with the first
/// subsystem that needs a key, and the loading and the redaction below are
/// finished and exercised without it.
#[derive(Debug)]
pub struct Config {
    settings: Vec<Setting>,
}

impl Config {
    /// The whole configuration as an operator should see it: every key, the
    /// value in force, and where that value came from, with every secret
    /// replaced.
    ///
    /// Redaction is done here, on the way out, rather than by remembering not to
    /// print a secret at each call site. There is one renderer, so there is one
    /// place for the rule to hold.
    pub fn effective(&self) -> String {
        let width = self
            .settings
            .iter()
            .map(|s| s.path.len())
            .max()
            .unwrap_or_default();

        let mut out = String::from("The configuration in force:\n");
        for setting in &self.settings {
            let shown = match (&setting.value, setting.secret) {
                (None, _) => "<unset>".to_owned(),
                (Some(_), true) => "<redacted>".to_owned(),
                (Some(value), false) => format!("{value:?}"),
            };
            let _ = writeln!(
                out,
                "  {:width$}  {shown}  ({})",
                setting.path,
                setting.origin.label(),
                width = width
            );
        }
        out
    }
}

/// What a load was asked to read.
pub struct Request {
    /// The file to read. `None` is a first start with no configuration file at
    /// all, which is not an error.
    pub file: Option<PathBuf>,
    /// True where the path above came from `--config` rather than from the
    /// default location. An operator who names a file and gets the defaults
    /// instead has been told nothing, so a named file that is not there is an
    /// error; the default location not being there is the ordinary case.
    pub named: bool,
    /// The environment, passed in rather than read, so the suite can drive it
    /// without touching the process it runs in.
    pub environment: BTreeMap<String, String>,
}

/// The file the server reads when nothing names one.
pub const DEFAULT_FILE: &str = "kartei.toml";

/// Read the file and the environment and produce the configuration in force, or
/// every reason it could not be produced.
pub fn load(request: &Request) -> Result<Config, Vec<Problem>> {
    let mut problems = Vec::new();
    let mut from_file: BTreeMap<String, String> = BTreeMap::new();

    if let Some(path) = &request.file {
        match std::fs::read_to_string(path) {
            Ok(text) => read_file(&text, path, &mut from_file, &mut problems),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && !request.named => {}
            Err(error) => problems.push(Problem {
                message: format!("could not read {}: {error}", path.display()),
            }),
        }
    }

    let mut settings = Vec::new();
    for k in SCHEMA {
        let variable = k.variable();
        let (value, origin) = match (
            request.environment.get(&variable),
            from_file.get(k.path),
            k.default,
        ) {
            (Some(v), _, _) => (Some(v.clone()), Origin::Environment),
            (None, Some(v), _) => (Some(v.clone()), Origin::File),
            (None, None, Some(d)) => (Some(d.to_owned()), Origin::Default),
            (None, None, None) => (None, Origin::Unset),
        };

        // Nested rather than a let chain. A let chain reads better and is
        // stable from 1.88, and the floor this workspace states in
        // `rust-version` is 1.85.0, so the shorter spelling would raise the
        // floor as a side effect of a line nobody looked at. The `msrv` job in
        // the gate is what turned that up here.
        if let Some(value) = &value {
            if let Err(reason) = (k.check)(value) {
                problems.push(Problem {
                    message: format!("{} is invalid: {reason}", k.path),
                });
            }
        }

        settings.push(Setting {
            path: k.path,
            value,
            origin,
            secret: k.secret,
        });
    }

    resolve_secret_files(&mut settings, &mut problems);

    if problems.is_empty() {
        Ok(Config { settings })
    } else {
        Err(problems)
    }
}

/// `admin.token` unset and `admin.token_file` set means the token is the
/// contents of that file.
///
/// The pairing is written out rather than derived from a naming convention. A
/// convention that turns `<key>_file` into `<key>` would make any future key
/// ending in `_file` a secret loader by accident, which is the kind of rule that
/// is discovered by a leak.
fn resolve_secret_files(settings: &mut [Setting], problems: &mut Vec<Problem>) {
    let path = settings
        .iter()
        .find(|s| s.path == "admin.token_file")
        .and_then(|s| s.value.clone());

    let Some(path) = path else { return };

    let Some(token) = settings.iter_mut().find(|s| s.path == "admin.token") else {
        return;
    };
    if token.value.is_some() {
        // The environment already carries it. The variable wins over the file
        // for the same reason it wins everywhere else: it is the narrower and
        // later statement of what the operator wants.
        return;
    }

    match std::fs::read_to_string(&path) {
        Ok(text) => {
            token.value = Some(text.trim_end_matches(['\n', '\r']).to_owned());
            token.origin = Origin::ReadFrom(path);
        }
        Err(error) => problems.push(Problem {
            message: format!("admin.token_file names {path}, which could not be read: {error}"),
        }),
    }
}

/// Flatten the document to dotted paths and judge each one.
fn read_file(
    text: &str,
    path: &Path,
    into: &mut BTreeMap<String, String>,
    problems: &mut Vec<Problem>,
) {
    let document = match DeTable::parse(text) {
        Ok(document) => document,
        Err(error) => {
            problems.push(Problem {
                message: format!("{} is not a readable file: {error}", path.display()),
            });
            return;
        }
    };

    let mut found = Vec::new();
    flatten(document.get_ref(), &mut String::new(), &mut found);

    for (dotted, value, at) in found {
        let line = line_of(text, at);
        match key(&dotted) {
            None => problems.push(Problem {
                message: unknown_key_message(&dotted, line),
            }),
            Some(k) if k.settable == Settable::EnvironmentOnly => problems.push(Problem {
                message: format!(
                    "{dotted} is set at line {line} and is not read from a file. \
                     A secret in the file is a secret in every copy of the file. \
                     Set {} instead, or point admin.token_file at a file holding it.",
                    k.variable()
                ),
            }),
            Some(_) => match value {
                Some(value) => {
                    into.insert(dotted, value);
                }
                None => problems.push(Problem {
                    message: format!(
                        "{dotted} at line {line} is not text. Every value this server \
                         reads is written in quotes."
                    ),
                }),
            },
        }
    }
}

/// Every leaf in the document, as a dotted path, its value where that value is
/// text, and the byte the key starts at.
fn flatten(
    table: &DeTable<'_>,
    prefix: &mut String,
    into: &mut Vec<(String, Option<String>, usize)>,
) {
    for (name, value) in table.iter() {
        let mark = prefix.len();
        if !prefix.is_empty() {
            prefix.push('.');
        }
        prefix.push_str(name.get_ref());

        match value.get_ref() {
            DeValue::Table(inner) => flatten(inner, prefix, into),
            DeValue::String(text) => into.push((
                prefix.clone(),
                Some(text.as_ref().to_string()),
                name.span().start,
            )),
            _ => into.push((prefix.clone(), None, name.span().start)),
        }

        prefix.truncate(mark);
    }
}

fn line_of(text: &str, at: usize) -> usize {
    text.as_bytes()[..at.min(text.len())]
        .iter()
        .filter(|b| **b == b'\n')
        .count()
        + 1
}

/// The message for a key nobody knows.
///
/// It always names the nearest known key, which is what the issue asks for, and
/// it says which of the two things it is doing. A suggestion offered for a key
/// nine edits away is worse than none, because it reads as a correction and
/// sends the operator to a key they never meant.
fn unknown_key_message(unknown: &str, line: usize) -> String {
    let (nearest, distance) = nearest_key(unknown);
    if distance <= 3 && distance < unknown.chars().count() {
        format!(
            "{unknown} at line {line} is not a key this server knows. \
             The nearest one it does know is {nearest}."
        )
    } else {
        format!(
            "{unknown} at line {line} is not a key this server knows. \
             The nearest one it does know is {nearest}, {distance} edits away, \
             which is far enough that it is probably not what was meant. \
             `kartei-server print-config` writes out every key."
        )
    }
}

/// The known key closest to `candidate`, and how far away it is.
fn nearest_key(candidate: &str) -> (&'static str, usize) {
    SCHEMA
        .iter()
        .map(|k| (k.path, distance(candidate, k.path)))
        // `min_by_key` keeps the first of equal elements, and SCHEMA has a
        // fixed order, so two keys the same distance away resolve the same way
        // on every run rather than on the iteration order of a map.
        .min_by_key(|(_, d)| *d)
        .expect("SCHEMA is never empty")
}

/// Levenshtein distance in characters.
///
/// Written here rather than depended on. It is fifteen lines, the key set is
/// five long, and a dependency taken for this would be a dependency in the
/// shipped binary for a suggestion in an error message.
fn distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();

    let mut previous: Vec<usize> = (0..=b.len()).collect();
    let mut current = vec![0; b.len() + 1];

    for (i, ca) in a.iter().enumerate() {
        current[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let substitute = previous[j] + usize::from(ca != cb);
            let delete = previous[j + 1] + 1;
            let insert = current[j] + 1;
            current[j + 1] = substitute.min(delete).min(insert);
        }
        std::mem::swap(&mut previous, &mut current);
    }

    previous[b.len()]
}

/// A configuration file carrying every key, its documentation and its default.
///
/// Emitted rather than committed. A file in the tree would be a second copy of
/// the key set, and the drift between the two would be invisible: both look
/// right on their own. `the_written_file_is_one_this_server_accepts` in the
/// suite loads what this produces, so the emitted file is exercised rather than
/// assumed to parse.
pub fn documented_file() -> String {
    let mut out = String::from(
        "# kartei configuration.\n\
         #\n\
         # Every key this server knows is below, with what it is for and the value\n\
         # in force when the key is absent. A key that is not in this list is an\n\
         # error at startup rather than a warning, so a misspelled key cannot\n\
         # quietly mean the default.\n\
         #\n\
         # Any key can be overridden by an environment variable, named in the\n\
         # comment above it.\n",
    );

    let mut section = "";
    for k in SCHEMA {
        let (table, leaf) = k.path.rsplit_once('.').unwrap_or(("", k.path));
        if table != section {
            let _ = write!(out, "\n[{table}]\n");
            section = table;
        }

        out.push('\n');
        for line in k.doc {
            let _ = writeln!(out, "# {line}");
        }
        let _ = writeln!(out, "# Overridden by {}.", k.variable());

        match (k.settable, k.default) {
            (Settable::EnvironmentOnly, _) => {
                let _ = writeln!(out, "# Not read from this file. {leaf} = ...");
            }
            (_, Some(default)) => {
                let _ = writeln!(out, "{leaf} = {default:?}");
            }
            (_, None) => {
                let _ = writeln!(out, "# No default. {leaf} = ...");
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_schema_gives_every_key_its_own_variable() {
        let mut seen: BTreeMap<String, &'static str> = BTreeMap::new();
        for k in SCHEMA {
            if let Some(other) = seen.insert(k.variable(), k.path) {
                panic!(
                    "{} and {} both override from {}, so setting it would move a key nobody \
                     named. The derivation replaces a dot with an underscore, so two paths \
                     differing only there collide.",
                    other,
                    k.path,
                    k.variable()
                );
            }
        }
    }

    #[test]
    fn every_key_says_what_it_is_for() {
        for k in SCHEMA {
            assert!(
                !k.doc.is_empty(),
                "{} carries no documentation, and the file this binary writes is the only \
                 place a key is documented",
                k.path
            );
        }
    }

    #[test]
    fn a_secret_is_never_readable_from_the_file() {
        for k in SCHEMA {
            if k.secret {
                assert_eq!(
                    k.settable,
                    Settable::EnvironmentOnly,
                    "{} is a secret and can be set in the file, so a configuration file \
                     cannot be shown to anybody",
                    k.path
                );
            }
        }
    }

    #[test]
    fn a_key_with_no_default_is_one_that_cannot_have_one() {
        // The two that have none are the administrator token and the file it
        // can be read from, and neither can be defaulted: inventing a token is
        // worse than not having one, and a path guessed on the operator's
        // behalf reads a file they did not name.
        let without: Vec<&str> = SCHEMA
            .iter()
            .filter(|k| k.default.is_none())
            .map(|k| k.path)
            .collect();
        assert_eq!(without, vec!["admin.token_file", "admin.token"]);
    }

    #[test]
    fn a_typo_one_character_long_is_pointed_at_the_key_that_was_meant() {
        let message = unknown_key_message("listen.addres", 2);
        assert!(
            message.contains("listen.address"),
            "the suggestion is the whole point of the message, and it said: {message}"
        );
        assert!(
            !message.contains("edits away"),
            "one character is close enough to suggest without a caveat: {message}"
        );
    }

    #[test]
    fn a_key_that_resembles_nothing_is_told_so_rather_than_sent_to_a_stranger() {
        let message = unknown_key_message("telemetry.endpoint", 7);
        assert!(
            message.contains("edits away"),
            "a suggestion offered for a distant key reads as a correction: {message}"
        );
        assert!(
            message.contains("print-config"),
            "where there is no useful suggestion the operator needs the list instead: {message}"
        );
    }

    #[test]
    fn the_nearest_key_does_not_depend_on_the_order_two_equal_candidates_are_seen_in() {
        // `min_by_key` keeps the first of equal elements and SCHEMA has a fixed
        // order, so this is a property of the code rather than of the data. The
        // leg exists because a change to a map-backed lookup would break it
        // silently, and an error message that differs between runs is one
        // nobody can quote.
        let first = nearest_key("nothing.like.a.key");
        for _ in 0..16 {
            assert_eq!(first, nearest_key("nothing.like.a.key"));
        }
    }

    #[test]
    fn the_distance_is_the_number_of_edits() {
        assert_eq!(distance("", ""), 0);
        assert_eq!(distance("address", "address"), 0);
        assert_eq!(distance("addres", "address"), 1);
        assert_eq!(distance("adress", "address"), 1);
        assert_eq!(distance("", "address"), 7);
    }

    #[test]
    fn a_value_the_key_will_not_accept_is_named_with_the_reason() {
        let reason = is_a_socket_address("8080").expect_err("a bare port is not an address");
        assert!(
            reason.contains("127.0.0.1:8080"),
            "the message has to show the shape that works, and said: {reason}"
        );

        let reason = is_a_log_level("chatty").expect_err("chatty is not a level");
        for level in LOG_LEVELS {
            assert!(
                reason.contains(level),
                "the message has to list the levels, and {level} was missing from: {reason}"
            );
        }
    }
}
