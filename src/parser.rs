//! Code to parse the duration grammar into milliseconds
//!
//! `parse_time` takes a string, and returns the duration that represents in milliseconds
//!
//! Examples of acceptable input:
//!
//! * `2 months, 5 day`
//! * `2weeks 5hrs` (commas are optional)
//! * `60secs`
//! * `5wk, 5d`
//! * `5weeks, 2weeks` (is additive, so this would result in 7 weeks)
//! * `60sec 2weeks` (order doesn't matter)
//!
//! See [the grammar](https://github.com/purarue/evry/blob/master/src/time.pest) for all
//! possible time durations.

// for pest macros
#![allow(clippy::upper_case_acronyms)]

use anyhow::{self, Result};
use pest::Parser;

/// The pest.rs Parser
#[derive(Parser)]
#[grammar = "time.pest"] // relative to src
pub struct TimeParser;

#[doc(hidden)]
const YEAR_MILLIS: u128 = 31556952000;
#[doc(hidden)]
const MONTH_MILLIS: u128 = 2592000000;
#[doc(hidden)]
const WEEK_MILLIS: u128 = 604800000;
#[doc(hidden)]
const DAY_MILLIS: u128 = 86400000;
#[doc(hidden)]
const HOUR_MILLIS: u128 = 3600000;
#[doc(hidden)]
const MINUTE_MILLIS: u128 = 60000;
#[doc(hidden)]
const SECOND_MILLIS: u128 = 1000;

/// uses macros to parse the pest.rs grammar into a duration (ms)
///
/// This parses the string into milliseconds.
pub fn parse_time(unparsed_input: &str) -> Result<u128, anyhow::Error> {
    let mut parsed_file = TimeParser::parse(Rule::file, unparsed_input)?;

    let mut total_millis: u128 = 0;

    // unwrap Rule::file, can't fail
    for line in parsed_file.next().unwrap().into_inner() {
        match line.as_rule() {
            Rule::durations => {
                // Pair { durations: [....] inner: [number, time unit] }
                for durations_expr in line.into_inner() {
                    //if debug {
                    //    println!("{:?}", durations_expr);
                    //}
                    let mut durations_inner = durations_expr.into_inner();
                    let quantity: u128 = durations_inner
                        .next() // item from inner rules
                        .unwrap()
                        .as_str() // numeric string, e.g. "3_000", "5"
                        .trim() // remove whitespace
                        .parse()
                        .expect("could not parse input into an integer");
                    let unit_str = durations_inner.next().unwrap();
                    // unwrap duration into string, parse again against Rule::singular,
                    // which doesn't consume the possible 's' from Rule::plural
                    // This looks a bit dangerous but its fine since pest is handling
                    // the erroneous input, we're just traversing the AST
                    let unit_millis: u128 =
                        match TimeParser::parse(Rule::singular, unit_str.as_str())
                            .ok() // parse result
                            .unwrap()
                            .next() // surrounding (singular/plural pair)
                            .unwrap()
                            .into_inner() // inner (unit specific) rules
                            .next() // first item (only item, the Rule::(singular) variant)
                            .unwrap()
                            .as_rule()
                        {
                            Rule::year => YEAR_MILLIS,
                            Rule::month => MONTH_MILLIS,
                            Rule::week => WEEK_MILLIS,
                            Rule::day => DAY_MILLIS,
                            Rule::hour => HOUR_MILLIS,
                            Rule::minute => MINUTE_MILLIS,
                            Rule::second => SECOND_MILLIS,
                            _ => unreachable!(),
                        };
                    // add the parsed duration to milliseconds
                    total_millis += unit_millis * quantity;
                }
            }
            // remove EOI
            Rule::EOI => (),
            _ => unreachable!(),
        };
    }
    Ok(total_millis)
}
