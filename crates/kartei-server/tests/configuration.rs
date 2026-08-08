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

//! The configuration is watched refusing, through the binary rather than
//! through the module.
//!
//! Every leg here starts the real executable, because what #62 asks about is
//! startup: whether a misspelled key stops the server, whether a first start
//! with no file at all comes up, and whether a secret can be read out of what
//! the server prints. Calling the loader directly would answer a question one
//! layer below the one being asked, and the layer in between is where an exit
//! code and a stream get chosen.
//!
//! ## What "never appears in any log line" means today
//!
//! The Done-when asks that a known secret never appears in the startup output
//! or in any log line. This binary has no logging subsystem yet: nothing writes
//! a log line, `log.level` is a key nothing reads, and the whole of what the
//! process emits is its standard output and its standard error. So the legs
//! below assert over both streams in full, and that is the whole of what is
//! checked. When logging lands it brings a third place a secret can reach, and
//! that place is not covered here.
//!
//! ## The environment
//!
//! Every variable starting with `KARTEI_` is removed from the child before each
//! run, and the ones a leg wants are then set. The removal is derived from the
//! parent's environment rather than from a list written here, so a key added to
//! the schema cannot leave a variable behind that this file does not know to
//! clear. Without it, a contributor who exports one of these for their own
//! server gets failures nobody else can reproduce.

use std::path::Path;
use std::process::Command;

use kartei_testing::TempDir;

/// A value no default and no message contains, so finding it in the output can
/// only mean it came from where the leg put it.
const SECRET: &str = "s3cret-tok3n-9f2a1c";

/// What one run of the binary produced.
struct Run {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

impl Run {
    /// Both streams, for the legs that care that something is absent from
    /// everything the process emitted rather than from one stream.
    fn everything(&self) -> String {
        format!("{}{}", self.stdout, self.stderr)
    }
}

/// Run the binary in `directory`, with `KARTEI_` cleared and `environment` set.
fn run(directory: &Path, arguments: &[&str], environment: &[(&str, &str)]) -> Run {
    let mut command = Command::new(env!("CARGO_BIN_EXE_kartei-server"));
    command.current_dir(directory).args(arguments);

    for (name, _) in std::env::vars() {
        if name.starts_with("KARTEI_") {
            command.env_remove(name);
        }
    }
    for (name, value) in environment {
        command.env(name, value);
    }

    let out = command
        .output()
        .expect("the binary under test is built by cargo before this target runs");

    Run {
        code: out.status.code(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

/// Write `text` into a file inside the test's own directory.
fn write(directory: &Path, name: &str, text: &str) -> std::path::PathBuf {
    let path = directory.join(name);
    std::fs::write(&path, text).expect("the test owns this directory");
    path
}

#[test]
fn an_unknown_key_stops_the_start_and_the_message_names_it_and_the_nearest_known_one() {
    let dir = TempDir::new("config-unknown-key");
    write(
        dir.path(),
        "kartei.toml",
        "[listen]\naddres = \"127.0.0.1:8080\"\n",
    );

    let out = run(dir.path(), &["--config", "kartei.toml"], &[]);

    assert_ne!(
        out.code,
        Some(0),
        "a misspelled key silently meaning the default is the failure this exists against, \
         and the run exited zero:\n{}",
        out.everything()
    );
    assert!(
        out.stderr.contains("listen.addres"),
        "the message has to name the key that was refused, and said:\n{}",
        out.stderr
    );
    assert!(
        out.stderr.contains("listen.address"),
        "the message has to name the nearest key that is known, and said:\n{}",
        out.stderr
    );
}

#[test]
fn a_first_start_with_no_file_at_all_succeeds() {
    let dir = TempDir::new("config-no-file");

    let out = run(dir.path(), &[], &[]);

    assert_eq!(
        out.code,
        Some(0),
        "a first start with no configuration file is the ordinary case:\n{}",
        out.everything()
    );
    assert!(
        out.stdout.contains("listen.address") && out.stdout.contains("(default)"),
        "the run has to say what it believes and where each value came from, and said:\n{}",
        out.stdout
    );
}

#[test]
fn a_file_the_operator_named_and_that_is_not_there_is_refused() {
    let dir = TempDir::new("config-named-absent");

    let out = run(dir.path(), &["--config", "not-here.toml"], &[]);

    assert_ne!(
        out.code,
        Some(0),
        "an operator who names a file and silently gets the defaults has been told nothing:\n{}",
        out.everything()
    );
    assert!(
        out.stderr.contains("not-here.toml"),
        "the message has to name the file it could not read, and said:\n{}",
        out.stderr
    );
}

#[test]
fn a_secret_set_in_the_environment_is_in_neither_stream() {
    let dir = TempDir::new("config-secret-environment");

    let out = run(dir.path(), &[], &[("KARTEI_ADMIN_TOKEN", SECRET)]);

    assert_eq!(
        out.code,
        Some(0),
        "the run should start:\n{}",
        out.everything()
    );
    assert!(
        !out.everything().contains(SECRET),
        "the token reached what the process printed:\n{}",
        out.everything()
    );
    assert!(
        out.stdout.contains("<redacted>"),
        "the key has to be shown as set and withheld rather than left out, and said:\n{}",
        out.stdout
    );
}

#[test]
fn a_secret_read_out_of_the_file_named_by_a_key_is_in_neither_stream() {
    let dir = TempDir::new("config-secret-file");
    let token = write(dir.path(), "token", &format!("{SECRET}\n"));
    write(
        dir.path(),
        "kartei.toml",
        &format!("[admin]\ntoken_file = '{}'\n", token.display()),
    );

    let out = run(dir.path(), &["--config", "kartei.toml"], &[]);

    assert_eq!(
        out.code,
        Some(0),
        "the run should start:\n{}",
        out.everything()
    );
    assert!(
        !out.everything().contains(SECRET),
        "the token reached what the process printed:\n{}",
        out.everything()
    );
    assert!(
        out.stdout.contains("read from"),
        "the operator has to be able to see where the value came from, and said:\n{}",
        out.stdout
    );
}

#[test]
fn the_secret_itself_is_refused_in_the_file() {
    let dir = TempDir::new("config-secret-in-file");
    write(
        dir.path(),
        "kartei.toml",
        &format!("[admin]\ntoken = \"{SECRET}\"\n"),
    );

    let out = run(dir.path(), &["--config", "kartei.toml"], &[]);

    assert_ne!(
        out.code,
        Some(0),
        "a secret in the file is a secret in every copy of the file:\n{}",
        out.everything()
    );
    assert!(
        !out.everything().contains(SECRET),
        "refusing the key must not print the value it was refusing:\n{}",
        out.everything()
    );
    assert!(
        out.stderr.contains("KARTEI_ADMIN_TOKEN"),
        "the message has to say where the value does belong, and said:\n{}",
        out.stderr
    );
}

#[test]
fn check_config_exits_non_zero_on_a_file_the_server_would_refuse() {
    let dir = TempDir::new("config-check-invalid");
    write(dir.path(), "kartei.toml", "[log]\nlevel = \"chatty\"\n");

    let out = run(
        dir.path(),
        &["check-config", "--config", "kartei.toml"],
        &[],
    );

    assert_ne!(
        out.code,
        Some(0),
        "a validate command that exits zero on a file the server refuses is worse than none:\n{}",
        out.everything()
    );
    assert!(
        out.stderr.contains("log.level"),
        "the message has to name the key whose value was refused, and said:\n{}",
        out.stderr
    );
}

#[test]
fn check_config_exits_zero_on_a_file_the_server_accepts_and_starts_nothing() {
    let dir = TempDir::new("config-check-valid");
    write(
        dir.path(),
        "kartei.toml",
        "[listen]\naddress = \"127.0.0.1:8080\"\n\n[log]\nlevel = \"debug\"\n",
    );

    let out = run(
        dir.path(),
        &["check-config", "--config", "kartei.toml"],
        &[],
    );

    assert_eq!(
        out.code,
        Some(0),
        "this file is one the server accepts:\n{}",
        out.everything()
    );
    assert!(
        out.stdout.contains("Nothing was started"),
        "a validate command has to say it validated rather than started, and said:\n{}",
        out.stdout
    );
    assert!(
        out.stdout.contains("\"debug\"") && out.stdout.contains("(configuration file)"),
        "the value from the file has to be the one in force, and the output said:\n{}",
        out.stdout
    );
}

#[test]
fn the_environment_overrides_the_file_for_the_same_key() {
    let dir = TempDir::new("config-override");
    write(dir.path(), "kartei.toml", "[log]\nlevel = \"debug\"\n");

    let out = run(
        dir.path(),
        &["--config", "kartei.toml"],
        &[("KARTEI_LOG_LEVEL", "trace")],
    );

    assert_eq!(
        out.code,
        Some(0),
        "both values are legal:\n{}",
        out.everything()
    );
    assert!(
        out.stdout.contains("\"trace\"") && out.stdout.contains("(environment)"),
        "the variable has to win over the file and say so, and the output said:\n{}",
        out.stdout
    );
    assert!(
        !out.stdout.contains("\"debug\""),
        "the value that lost must not be shown as the value in force:\n{}",
        out.stdout
    );
}

#[test]
fn the_written_file_is_one_this_server_accepts() {
    let dir = TempDir::new("config-round-trip");

    let written = run(dir.path(), &["print-config"], &[]);
    assert_eq!(
        written.code,
        Some(0),
        "print-config should write a file:\n{}",
        written.everything()
    );
    write(dir.path(), "kartei.toml", &written.stdout);

    let out = run(
        dir.path(),
        &["check-config", "--config", "kartei.toml"],
        &[],
    );

    assert_eq!(
        out.code,
        Some(0),
        "the documented file this binary writes has to be one it reads back:\n{}\n\
         the file it wrote was:\n{}",
        out.everything(),
        written.stdout
    );
}

#[test]
fn print_config_answers_even_where_the_configuration_would_be_refused() {
    let dir = TempDir::new("config-print-despite-refusal");
    write(
        dir.path(),
        "kartei.toml",
        "[listen]\naddres = \"nonsense\"\n",
    );

    let out = run(
        dir.path(),
        &["print-config", "--config", "kartei.toml"],
        &[],
    );

    assert_eq!(
        out.code,
        Some(0),
        "a broken file is the likeliest reason to ask what the keys are:\n{}",
        out.everything()
    );
    assert!(
        out.stdout.contains("listen.address") || out.stdout.contains("address ="),
        "the written file has to carry the keys, and said:\n{}",
        out.stdout
    );
}

#[test]
fn an_option_the_binary_does_not_know_exits_differently_from_a_refused_configuration() {
    let dir = TempDir::new("config-unknown-option");

    let out = run(dir.path(), &["--confg", "kartei.toml"], &[]);

    assert_eq!(
        out.code,
        Some(2),
        "a command line nobody can read is a different failure from a file the server \
         refuses, and telling them apart is what lets a script react to each:\n{}",
        out.everything()
    );
}
