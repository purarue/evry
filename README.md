# evry

A shell-script-centric task scheduler; uses exit codes to determine control flow. Most of the time I call this behind [bgproc](https://github.com/purarue/bgproc).

- [Install](#install)
- [Rationale](#rationale)
- [Duration Examples](#duration)
- [Examples](#examples)
- [Advanced Usage](#advanced-usage)

### Install

Install `rust`/`cargo`, then:

```
cargo install evry
```

## Rationale

```
A tool to manually run commands -- periodically.
Uses shell exit codes to determine control flow in shell scripts

Usage:
  evry <describe duration>... <-tagname>
  evry location <-tagname>
  evry duration <describe duration...>
  evry help
```

Best explained with an example:

`evry 2 weeks -scrapesite && wget "https://" -o ....`

In other words, run the `wget` command every `2 weeks`.

`evry` exits with an unsuccessful exit code if the command has been run in the last `2 weeks` (see below for more duration examples), which means the `wget` command wouldn't run.

When `evry` exits with a successful exit code, it saves the current time to a metadata file for that tag (`-scrapesite`). That way, when `evry` is run again with that tag, it can compare the current time against that file.

This can _sort of_ be thought of as `cron` alternative, but operations don't run in the background. It requires you to call the command yourself, but it won't run if its already run in the time frame you describe. (However, its not difficult to wrap tasks that run behind `evry` in an infinite loop that runs in the background, which is what [`bgproc`](https://github.com/purarue/bgproc) does)

You could have an infinite loop running in the background like:

```bash
while true; do
  evry 1 month -runcommand && run command
  sleep 60
done
```

... and even though that tries to run the command every 60 seconds, `evry` exits with an unsuccessful exit code, so `run command` would only get run once per month.

The `-runcommand` is just an arbitrary tag name so that `evry` can save metadata about a command to run/job. Its only use is to uniquely identify some task, and save a metadata file to your [local data directory](https://docs.rs/app_dirs/1.2.1/app_dirs/). If you want to overwrite the default location, you can set the `EVRY_DIR` variable. E.g., in your shell profile:

```bash
export EVRY_DIR="$HOME/.local/share/tags"
```

Since this doesn't run in a larger context and `evry` can't know if a command failed to run - if a command fails, you can remove the tag file, to reset it to run again later (since if the file doesn't exist, `evry` assumes its a new task):

```bash
evry 2 months -selenium && {
# evry succeeded, so the external command should be run
    python selenium.py || {
        # the python process exited with a non-zero exit code
        # remove the tag file so we can re-try later
        rm "$(evry location -selenium)"
        # maybe notify you that this failed so you go and check on it
        notify-send -u critical 'selenium failed!"
    }
}
```

### Duration

The duration (e.g. `evry 2 months, 5 days`) is parsed with a [`PEG`](https://en.wikipedia.org/wiki/Parsing_expression_grammar), so its very flexible. All of these are valid duration input:

- `2 months, 5 day`
- `2weeks 5hrs` (commas are optional)
- `60secs`
- `5wk, 5d`
- `5weeks, 2weeks` (is additive, so this would result in 7 weeks)
- `60sec 2weeks` (order doesn't matter)

See [the grammar](https://github.com/purarue/evry/blob/5a98d5607654c90a43eb02ee3304d3bcae1a9a3a/src/time.pest#L5-L11) for all possible abbreviations.

This also includes a utility `duration` command to print a parsed duration in seconds:

```
$ evry duration 5m
300
$ evry duration 5 minutes
300
$ evry duration 10 days
864000
```

Can run with `EVRY_JSON=1` to print JSON with more formats.

### Examples

This could be used to do anything you might use anacron for. For example, to periodically sync files:

```bash
evry 1d -backup && rsync ...
```

Or, cache the output of a command, once a day (e.g. my [`jumplist`](https://github.com/purarue/dotfiles/blob/baf92d5fed00b87167b509f22d439c5e2075f63b/.local/scripts/generic/jumplist))

```bash

expensive_command_cached() {
	evry 1d -expensive_command_cached && cmd >~/.cache/cmd_output
	cat ~/.cache/cmd_output
}

expensive_command_cached
```

I have certain jobs (e.g. scraping websites for metadata, using [`selenium`](https://www.selenium.dev/) to [login to some website and click a button](https://github.com/purarue/pythonanywhere-3-months), or [checking my music for metadata](https://github.com/purarue/plaintext_playlist_py) that I want to run periodically.

Putting all my jobs I want to run periodically in one [`housekeeping`](https://github.com/purarue/dotfiles/blob/master/.local/scripts/linux/housekeeping) script I run daily/weekly gives me the ability to monitor the output easily, but also allows me the flexibility of being able to schedule tasks to run at different rates. It also means that those scripts/commands can prompt me for input/confirmation, since this is run manually from a terminal, not in the background like cron.

I often use this instead of cron when developing websites, e.g. [here](https://github.com/purarue/dbsentinel/blob/32b81d09b201a92f7308ceda0b4323eff52b7df5/update_data#L97-L115), where I use it to periodically run caching tasks for a webservice. Having them in a script like this means its the same interface/environment while I'm developing and deploying, so there's no issues with possibly missing environment variables/being in the wrong directory when deploying to production, and its easy to 'reset' a cron job while I'm developing

### Advanced Usage

The `EVRY_DEBUG` environment variable can be set to provide information on what was parsed from user input, and how long till the next run succeeds.

```
$ EVRY_DEBUG=1 evry 2 months -pythonanywhere && pythonanywhere_3_months -Hc "$(which chromedriver)"
tag_name:pythonanywhere
data_directory:/home/username/.local/share/evry/data
log:parsed '2 months' into 5184000000ms
log:60 days (5184000000ms) haven't elapsed since last run, exiting with code 1
log:Will next be able to run in '46 days, 16 hours, 46 minutes, 6 seconds' (4034766587ms)
```

The `EVRY_PARSE_ERROR_LOG` environment variable can be set to save any duration parsing errors to a file, which can be useful for debugging, especially if you're dynamically generating the duration string. In your shell profile:

```bash
export EVRY_PARSE_ERROR_LOG="$HOME/.cache/evry_parse_errors.log"
```

If you wanted to 'reset' a task, you could do: `rm ~/.local/share/evry/data/<tag name>`; removing the tag file. The next time that `evry` runs, it'll assume its a new task, and exit successfully. I use the following shell function (see [`functions.sh`](./functions.sh)) to 'reset' tasks:

```bash
job-reset() {
	local EVRY_DATA_DIR
	EVRY_DATA_DIR="$(evry location - 2>/dev/null)"
	cd "${EVRY_DATA_DIR}"
	fzf -q "$*" -m | while read -r tag; do
		rm -v "${tag}"
	done
	cd -
}
```

The `EVRY_JSON` environment variable can be set to provide similar information in a more consumable format (e.g. with [`jq`](https://github.com/stedolan/jq))

As an example; `./schedule_task`:

```bash
#!/bin/bash

if JSON_OUTPUT="$(EVRY_JSON=1 evry 2 hours -task)"; then
  echo "Running task..."
else
  # extract the body for a particular log message
  NEXT_RUN="$(echo "$JSON_OUTPUT" | jq -r '.[] | select(.type == "till_next_pretty") | .body')"
  printf 'task will next run in %s\n' "$NEXT_RUN"
fi
```

```
$ ./schedule_task
Running task...
$ ./schedule_task
task will next run in 1 hours, 59 minutes, 58 seconds
```

For reference, typical JSON output when `evry` fails (command doesn't run):

```json
[
  {
    "type": "tag_name",
    "body": "task"
  },
  {
    "type": "data_directory",
    "body": "/home/username/.local/share/evry/data"
  },
  {
    "type": "log",
    "body": "parsed '2 hours' into 7200000ms"
  },
  {
    "type": "duration",
    "body": "7200000"
  },
  {
    "type": "duration_pretty",
    "body": "2 hours"
  },
  {
    "type": "log",
    "body": "2 hours (7200000ms) haven't elapsed since last run, exiting with code 1"
  },
  {
    "type": "log",
    "body": "Will next be able to run in '1 hours, 58 minutes, 17 seconds' (7097748ms)"
  },
  {
    "type": "till_next",
    "body": "7097748"
  },
  {
    "type": "till_next_pretty",
    "body": "1 hours, 58 minutes, 17 seconds"
  }
]
```

If you want to extract multiple values from the output which have distinct keys (e.g. `till_next_pretty`, `till_next` and `tag_name`), you might find this snippet useful:

```bash
$ EVRY_JSON=1 evry 12 hours -bleanser-zsh | jq '[.[] | {key: .type, value: .body}] | from_entries'
{
  "tag_name": "bleanser-zsh",
  "data_directory": "/home/username/.local/share/evry/data",
  "log": "Tag file doesn't exist, creating and exiting with code 0",
  "duration": "43200000",
  "duration_pretty": "12 hours"
}
```

You could then use `jq` to extract the `duration`/`duration_pretty`/`tag_name` values from that output, like:

```bash
$ OUT="$(EVRY_JSON=1 evry 12 hours -bleanser-zsh | jq '[.[] | {key: .type, value: .body}] | from_entries')"
$ echo "$OUT" | jq -r '.duration'
43200000
$ echo "$OUT" | jq -r '.duration_pretty'
12 hours
```
