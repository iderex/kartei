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

//! The binary: HTTP, sockets, configuration, and the assembly of the crates
//! that do the work.
//!
//! This is the only crate that depends on every other one, because it is the
//! only crate that is allowed to know how they fit together. It is also where
//! an engine is selected, by enabling the `engine` feature on `kartei-sync`.
//!
//! What is assembled today is the configuration and nothing else. The parts
//! land with their own milestones, and the layout exists first so that each one
//! has a place to land in and a dependency graph that can be checked before it
//! does. `run` therefore reads the configuration, says what it read and stops,
//! and it prints that it stopped for that reason rather than exiting silently
//! and letting a zero be read as a server that is up.
//!
//! ## The three verbs
//!
//! `run` is the default and is what a container's entrypoint calls. It loads
//! the configuration and prints the configuration in force, with every secret
//! replaced.
//!
//! `check-config` loads the same configuration and starts nothing, so an
//! operator can find out whether a file is acceptable without taking a service
//! down to find out. It exits non-zero on a file the server would refuse, which
//! is what makes it usable from a deployment script.
//!
//! `print-config` writes a configuration file carrying every key, what it is
//! for, its default and the environment variable that overrides it. It is
//! generated from the same key list the loader reads, so the documentation of a
//! key cannot drift away from the key.
//!
//! ## Exit codes
//!
//! `0` for a verb that did what it was asked. `1` for a configuration the
//! server refuses. `2` for a command line it cannot read, which is a different
//! failure and is worth telling apart from the configuration being wrong.

mod config;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;

/// A configuration the server refuses.
const REFUSED: u8 = 1;
/// A command line the binary cannot read.
const UNREADABLE_COMMAND_LINE: u8 = 2;

const USAGE: &str = "\
kartei-server [VERB] [--config PATH]

Verbs:
  run             Read the configuration, print what is in force, and stop.
                  Nothing is served yet. This is the default.
  check-config    Read the configuration and start nothing. Exits non-zero
                  where the server would refuse the file.
  print-config    Write a configuration file carrying every key, what it is
                  for, its default, and the variable that overrides it.

Options:
  --config PATH   The configuration file to read. Without this the server
                  reads ./kartei.toml where it is present, and a first start
                  with no file at all is not an error.
";

/// What the command line asked for.
#[derive(PartialEq, Eq, Debug)]
enum Verb {
    Run,
    CheckConfig,
    PrintConfig,
}

/// The command line, once it has been read.
#[derive(Debug)]
struct Invocation {
    verb: Verb,
    /// The file to read, and whether the operator named it.
    file: Option<PathBuf>,
    named: bool,
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let environment: BTreeMap<String, String> = std::env::vars().collect();

    let invocation = match read_command_line(&arguments) {
        Ok(invocation) => invocation,
        Err(reason) => {
            eprintln!("{reason}\n\n{USAGE}");
            return ExitCode::from(UNREADABLE_COMMAND_LINE);
        }
    };

    if invocation.verb == Verb::PrintConfig {
        // Deliberately before the load. A file that will not load is the
        // likeliest reason somebody asks for this one, so needing a working
        // configuration to be told what the keys are would withhold the answer
        // exactly when it is wanted.
        print!("{}", config::documented_file());
        return ExitCode::SUCCESS;
    }

    let request = config::Request {
        file: invocation.file,
        named: invocation.named,
        environment,
    };

    let loaded = match config::load(&request) {
        Ok(loaded) => loaded,
        Err(problems) => {
            eprintln!(
                "The configuration was refused, {} problem(s):",
                problems.len()
            );
            for problem in &problems {
                eprintln!("  {}", problem.message);
            }
            return ExitCode::from(REFUSED);
        }
    };

    match invocation.verb {
        Verb::PrintConfig => unreachable!("returned above"),
        Verb::CheckConfig => {
            print!("{}", loaded.effective());
            println!("This configuration is one the server accepts. Nothing was started.");
        }
        Verb::Run => {
            print!("{}", loaded.effective());
            println!(
                "Nothing is served yet: this build assembles the configuration and no other \
                 part. The configuration above was read and accepted, and no socket was bound."
            );
        }
    }

    ExitCode::SUCCESS
}

/// Read the arguments, refusing anything not understood.
///
/// An unknown flag is an error rather than something ignored. A flag that is
/// ignored is the same failure this whole module exists against, one level up:
/// `--confg` silently means the default file, and the operator is looking at
/// output from a file they did not name.
fn read_command_line(arguments: &[String]) -> Result<Invocation, String> {
    let mut verb = None;
    let mut file = None;
    let mut rest = arguments.iter();

    while let Some(argument) = rest.next() {
        if let Some(value) = argument.strip_prefix("--config=") {
            file = Some(PathBuf::from(value));
        } else if argument == "--config" {
            match rest.next() {
                Some(value) => file = Some(PathBuf::from(value)),
                None => return Err("--config needs the path of a file after it.".to_owned()),
            }
        } else if argument == "--help" || argument == "-h" {
            // Not an error, and not a verb either: it answers and stops.
            print!("{USAGE}");
            std::process::exit(0);
        } else if argument.starts_with('-') {
            return Err(format!("{argument} is not an option this binary knows."));
        } else if verb.is_some() {
            return Err(format!(
                "{argument} is a second verb, and this binary takes one."
            ));
        } else {
            verb = Some(match argument.as_str() {
                "run" => Verb::Run,
                "check-config" => Verb::CheckConfig,
                "print-config" => Verb::PrintConfig,
                other => return Err(format!("{other} is not a verb this binary knows.")),
            });
        }
    }

    let named = file.is_some();
    Ok(Invocation {
        verb: verb.unwrap_or(Verb::Run),
        file: file.or_else(|| Some(PathBuf::from(config::DEFAULT_FILE))),
        named,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(arguments: &[&str]) -> Result<Invocation, String> {
        let owned: Vec<String> = arguments.iter().map(|a| (*a).to_owned()).collect();
        read_command_line(&owned)
    }

    #[test]
    fn no_arguments_is_run_against_the_default_file_which_may_be_absent() {
        let invocation = line(&[]).expect("no arguments is legal");
        assert_eq!(invocation.verb, Verb::Run);
        assert_eq!(invocation.file, Some(PathBuf::from(config::DEFAULT_FILE)));
        assert!(
            !invocation.named,
            "the default file was not named by the operator, so its absence is not an error"
        );
    }

    #[test]
    fn a_named_file_is_recorded_as_named_in_both_spellings() {
        for arguments in [
            vec!["--config", "somewhere/kartei.toml"],
            vec!["--config=somewhere/kartei.toml"],
        ] {
            let invocation = line(&arguments).expect("both spellings are legal");
            assert_eq!(
                invocation.file,
                Some(PathBuf::from("somewhere/kartei.toml"))
            );
            assert!(
                invocation.named,
                "{arguments:?} named a file, so its absence has to be an error"
            );
        }
    }

    #[test]
    fn an_option_this_binary_does_not_know_is_refused_rather_than_ignored() {
        let reason = line(&["--confg", "kartei.toml"]).expect_err("--confg is not an option");
        assert!(
            reason.contains("--confg"),
            "the message has to name what was refused, and said: {reason}"
        );
    }

    #[test]
    fn a_verb_this_binary_does_not_know_is_refused() {
        let reason = line(&["serve"]).expect_err("serve is not a verb");
        assert!(
            reason.contains("serve"),
            "the message has to name what was refused, and said: {reason}"
        );
    }

    #[test]
    fn each_verb_is_read_back() {
        assert_eq!(line(&["run"]).expect("run").verb, Verb::Run);
        assert_eq!(
            line(&["check-config"]).expect("check-config").verb,
            Verb::CheckConfig
        );
        assert_eq!(
            line(&["print-config"]).expect("print-config").verb,
            Verb::PrintConfig
        );
    }
}
