use std::env;
use std::process::exit;

extern crate pest;
#[macro_use]
extern crate pest_derive;

mod app_path;
mod parser;

#[derive(Debug)]
pub struct CLI {
    args: Vec<String>,
    debug: bool,
    rollback: bool,
    tag: app_path::Tag,
}

impl CLI {
    fn help(warn: bool) {
        if warn {
            println!("Not enough arguments provided.");
        }
        println!(
            "A tool to manually run commands -- periodically.
Uses shell exit codes to determine control flow in shell scripts

Usage:
  evry [describe duration]... <-tagname>
  evry rollback <-tagname>
  evry help


Best explained with an example:

evry 2 weeks -scrapesite && wget \"https://\" -o ....

In other words, run the wget command every 2 weeks.

evry exits with an unsuccessful exit code if the command has
been run in the last 2 weeks, which means the wget command wouldn't run.

When evry exits with a successful exit code, it saves the current time
to a metadata file for that tag (-scrapesite). That way, when evry
is run again with that tag, it can compare the current time against that file.

rollback is used incase the command failed to run, see
https://github.com/seanbreckenridge/evry for more examples.
"
        );
        exit(2)
    }

    pub fn parse_args(dir_info: &app_path::LocalDir) -> Self {
        // get arguments (remove binary name)
        let args: Vec<String> = env::args().skip(1).collect();
        let args_len = args.len();
        // if user asked for help
        if args_len >= 1 && (args[0] == "help" || args[0] == "--help") {
            CLI::help(false)
        }
        // split CLI arguments into tag/other strings
        let (tag_vec, other_vec): (_, Vec<_>) =
            args.into_iter().partition(|arg| arg.starts_with('-'));
        // user didn't provide argument
        if tag_vec.is_empty() || other_vec.is_empty() {
            CLI::help(true)
        }
        // parse tag, remove the '-' from the name
        let tag_raw = &tag_vec[0];
        let tag = tag_raw
            .chars()
            .next()
            .map(|c| &tag_raw[c.len_utf8()..])
            .expect("Couldn't parse tag from arguments");
        // if asked to rollback
        let rollback = &other_vec[0] == "rollback";
        CLI {
            args: other_vec,
            debug: env::var("EVRY_DEBUG").is_ok(),
            rollback,
            tag: app_path::Tag::new(tag.to_string(), dir_info),
        }
    }
}

fn main() {
    // global application information
    let dir_info = app_path::LocalDir::new();
    let cli = CLI::parse_args(&dir_info);
    if cli.debug {
        println!("{:?}", dir_info);
        println!("{:?}", cli);
    }

    if cli.rollback {
        if cli.debug {
            println!("Running rollback...");
        }
        app_path::restore_rollback(&dir_info, &cli.tag);
        exit(0)
    }

    // parse duration string
    let run_every = parser::parse_time(cli.args, cli.debug);

    if cli.debug {
        println!("Parsed input date string into {} milliseconds", run_every);
    }

    if !cli.tag.file_exists() {
        // file doesn't exist, this is the first time this tag is being run.
        // save the current milliseconds to the file and exit with a 0 exit code
        if cli.debug {
            println!("Tag file doesn't exist, creating and exiting successfully.")
        }
        cli.tag.write_epoch_millis();
        exit(0)
    } else {
        // file exists, read last time this tag was run
        let last_ran_at = cli.tag.read_epoch_millis();
        if app_path::epoch_time() - last_ran_at > run_every {
            // duration this should be run at has elapsed, run
            if cli.debug {
                println!("Has been more than {} milliseconds, saving to rollback file and writing to tag file", run_every)
            }
            // dump this to rollback file so it can this can be rolled back if external command fails
            app_path::save_rollback(&dir_info, last_ran_at);
            // save current time to tag file
            cli.tag.write_epoch_millis();
            exit(0)
        } else {
            // this has been run within the specified duration, don't run
            if cli.debug {
                println!("Hasn't been more than {} milliseconds since last run according to tag file, failing (exit code 1)", run_every)
            }
            exit(1)
        }
    }
}
