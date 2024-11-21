#![warn(missing_docs)]
#![warn(missing_doc_code_examples)]
#![allow(clippy::needless_return)]

//! A shell-script-centric task scheduler; uses exit codes to determine control flow.
//!
//! Best explained with an example:
//!
//! `evry 2 weeks -scrapesite && wget "https://" -o ....`
//!
//! In other words, run the `wget` command every `2 weeks`.
//!
//! `evry` exits with an unsuccessful exit code if the command has been run in the last `2 weeks` (see the parser module for more examples), which means the `wget` command wouldn't run.
//!
//! When `evry` exits with a successful exit code, it saves the current time to a metadata file for that tag (`-scrapesite`). That way, when `evry` is run again with that tag, it can compare the current time against that file.
//!
//! This can *sort of* be thought of as `cron` alternative, but operations don't run in the background. It requires you to call the command yourself, but it won't run if its already run in the time frame you describe.
//!
//! You could have an infinite loop running in the background like:
//!
//! ```bash
//! while true; do
//!   evry 1 month -runcommand && run command
//!   sleep 60
//! done
//! ```
//!
//! ... and even though that tries to run the command every 60 seconds, `evry` exits with an unsuccessful exit code, so `run command` would only get run once per month.
//!
//! The `-runcommand` is just an arbitrary tag name so that `evry` can save metadata about a command to run/job. Can be chosen arbitrarily, its only use is to uniquely identify runs of `evry`, and save a metadata file to your [local data directory](https://docs.rs/app_dirs/1.2.1/app_dirs/)
//!
//! Since this doesn't run in a larger context and its just a bash script, if a command fails, you can remove the tag file, to reset it to run again later (since if the file doesn't exist, `evry` assumes its a new task)

use std::env;
use std::io::Write;
use std::process::exit;
use std::string::String;

use anyhow::{Context, Error, Result};
extern crate pest;
#[macro_use]
extern crate pest_derive;

mod file;
mod parser;
mod printer;
mod utils;

#[derive(Debug)]
enum Command {
    Location,
    Duration,
    Run,
}

/// parses the user input; flags/environment variables
#[derive(Debug)]
struct Args {
    /// unparsed, string representation of a date from the user
    raw_date: String,
    /// if EVRY_DEBUG=1 was set
    debug: bool,
    /// if EVRY_JSON=1 was set
    json: bool,
    // if the user wants to print location/duration instead of running normally
    command: Command,
    /// tagfile to read/write from, uniquely identifies this job
    tag: file::Tag,
}

impl Args {
    /// prints the help message
    fn help() {
        println!(
            "A tool to manually run commands -- periodically.
Uses shell exit codes to determine control flow in shell scripts

Usage:
  evry <describe duration>... <-tagname>
  evry location <-tagname>
  evry duration <describe duration...>
  evry help

Best explained with an example:

evry 2 weeks -scrapesite && wget \"https://\" -o ....

In other words, run the wget command every 2 weeks.

evry exits with an unsuccessful exit code if the command has
been run in the last 2 weeks, which means the wget command wouldn't run.

When evry exits with a successful exit code, it saves the current time
to a metadata file for that tag (-scrapesite). That way, when evry
is run again with that tag, it can compare the current time against that file.

location prints the computed tag file location

duration just lets you use this as a duration parser, without interacting with the filesystem
it prints the parsed duration in seconds. Running with JSON mode prints more formats

See https://github.com/purarue/evry for more examples."
        );
        // exit with an unsuccessful exit code so if user is doing some complex argparsing
        // in a bash script, and this fails to parse the arguments,
        // this fails and doesn't run the dependent command accidentally
        exit(10);
    }

    /// parses command-line user input/environment variables
    fn parse_args(dir_info: &file::LocalDir) -> Result<Self, Error> {
        // get arguments (remove binary name)
        let args: Vec<String> = env::args().skip(1).collect();
        // if user asked for help
        if args
            .iter()
            .filter(|&arg| arg == "help" || arg == "--help")
            .count()
            > 0
        {
            Args::help()
        }
        // split args arguments into tag/other strings
        let (tag_vec, other_vec): (_, Vec<_>) =
            args.into_iter().partition(|arg| arg.starts_with('-'));
        if other_vec.is_empty() {
            eprintln!("Error: Must provide a duration string or a command\n");
            Args::help()
        }
        let first_arg = &other_vec[0];
        let command: Command = match first_arg.as_str() {
            "location" => Command::Location,
            "duration" => Command::Duration,
            _ => Command::Run,
        };
        let date_string = match command {
            Command::Location | Command::Duration => other_vec[1..].join(" "),
            _ => other_vec.join(" "),
        };
        if tag_vec.is_empty() && !matches!(command, Command::Duration) {
            eprintln!("Error: Must provide a tag name using a hyphen or a command\n");
            Args::help()
        }
        // parse tag, remove the first character ('-') from the tag
        let tag: String = tag_vec
            .iter()
            .map(|arg| arg.chars().skip(1).collect::<String>())
            .collect::<Vec<String>>()
            .join("_");
        // if user didn't ask for duration, they have to provide a tag
        if tag.chars().count() == 0 && first_arg != "duration" {
            eprintln!("Error: passed tag was an empty string\n");
        }
        match command {
            Command::Location => (),
            _ => {
                if date_string.chars().count() == 0 {
                    eprintln!("Error: passed duration was an empty string");
                    Args::help()
                }
            }
        }
        let json = env::var("EVRY_JSON").is_ok();
        Ok(Args {
            command,
            raw_date: date_string,
            // specifying EVRY_JSON automatically enables debug as well
            // otherwise evry is supposed to remain silent -- its not meant to print anything
            debug: json | env::var("EVRY_DEBUG").is_ok(),
            json,
            tag: file::Tag::new(tag.to_string(), dir_info),
        })
    }
}

/// encapsulates the logic for evry, printing logs to the printer
/// if debug is enabled.
/// Returns an exit code to signify what to do
fn evry(dir_info: file::LocalDir, cli: Args, printer: &mut printer::Printer) -> Result<i32, Error> {
    if cli.debug {
        printer.echo("tag_name", &cli.tag.name);

        let dir_path: String = dir_info.data_dir.into_os_string().into_string().unwrap();
        printer.echo("data_directory", &dir_path);
    }

    if matches!(cli.command, Command::Location) {
        // causes an early exit, print directly instead of using the printer
        // user is probably trying to use this to compute the location like
        // SHELLVAR="$(evry location -tagname)"
        println!("{}", cli.tag.path);
        return Ok(0);
    }

    // parse duration string
    let run_every = match parser::parse_time(&cli.raw_date) {
        Ok(time) => time,
        Err(_e) => {
            printer.echo(
                "error",
                &format!("couldn't parse '{}' into a duration", cli.raw_date),
            );
            if let Ok(evry_parse_logfile) = env::var("EVRY_PARSE_ERROR_LOG") {
                let mut logfile = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(evry_parse_logfile)
                    .context("Couldn't open EVRY_PARSE_ERROR_LOG")?;
                writeln!(
                    logfile,
                    "Could not parse: {} -{}",
                    cli.raw_date, cli.tag.name
                )
                .context("Couldn't write to logfile")?;
            }
            return Ok(1); // fatal error
        }
    };

    if matches!(cli.command, Command::Duration) {
        if !cli.debug {
            println!("{}", run_every / 1000);
        } else {
            printer.echo("duration", &format!("{}", run_every));
            printer.echo("duration_seconds", &format!("{}", run_every / 1000));
            printer.echo("duration_pretty", &utils::describe_ms(run_every));
        }
        return Ok(0);
    }

    // get current time
    let now = utils::epoch_millis().context("Couldn't get current time")?;

    if cli.debug {
        printer.echo(
            "log",
            &format!("parsed '{}' into {}ms", cli.raw_date, run_every),
        );
        printer.print(
            printer::Message::new("duration", &format!("{}", run_every)),
            Some(printer::PrinterType::Json),
        );
        printer.print(
            printer::Message::new("duration_pretty", &utils::describe_ms(run_every)),
            Some(printer::PrinterType::Json),
        );
    }

    if !cli.tag.file_exists() {
        // file doesn't exist, this is the first time this tag is being run.
        // save the current milliseconds to the file and exit with a 0 exit code
        if cli.debug {
            printer.echo(
                "log",
                "Tag file doesn't exist, creating and exiting with code 0",
            );
        }
        cli.tag.write(now)?;
        return Ok(0);
    } else {
        // file exists, read last time this tag was run
        let last_ran_at = cli.tag.read_epoch_millis()?;
        if now - last_ran_at > run_every {
            // duration this should be run at has elapsed, run
            if cli.debug {
                printer.echo("log", &format!("Has been more than '{}' ({}ms) since last succeeded, writing to tag file, exiting with code 0", utils::describe_ms(run_every), run_every));
            }
            // save current time to tag file
            cli.tag.write(now)?;
            return Ok(0);
        } else {
            // this has been run within the specified duration, don't run
            if cli.debug {
                printer.echo(
                    "log",
                    &format!(
                        "{} ({}ms) haven't elapsed since last run, exiting with code 1",
                        utils::describe_ms(run_every),
                        run_every
                    ),
                );
                let till_next_run = last_ran_at + run_every - now;
                let till_next_pretty = utils::describe_ms(till_next_run);
                printer.echo(
                    "log",
                    &format!(
                        "Will next be able to run in '{}' ({}ms)",
                        till_next_pretty, till_next_run
                    ),
                );
                printer.print(
                    printer::Message::new("till_next", &format!("{}", till_next_run)),
                    Some(printer::PrinterType::Json),
                );
                printer.print(
                    printer::Message::new("till_next_pretty", &till_next_pretty),
                    Some(printer::PrinterType::Json),
                );
            }
            return Ok(2); // exit code 2; expected error, to cause next shell command to not run
        }
    }
}

fn main() -> Result<(), Error> {
    // global application information
    let dir_info = file::LocalDir::new()?;
    let cli = Args::parse_args(&dir_info)?;

    let printer_type = if cli.json {
        printer::PrinterType::Json
    } else {
        printer::PrinterType::Stderr
    };

    // handles printing/saving messages in case we're in JSON mode
    let mut printer = printer::Printer::new(printer_type);

    // run 'main' code, saving exit code
    let result = evry(dir_info, cli, &mut printer)?;

    // if user specified JSON, print the blob
    printer.flush();
    exit(result);
}
