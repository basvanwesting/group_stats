mod number_stats;
mod output_number_data;
mod output_row;
mod output_string_data;
mod string_stats;

use clap::{CommandFactory, Parser};
use is_terminal::IsTerminal as _;
use number_stats::NumberStats;
use output_number_data::OutputNumberData;
use output_string_data::OutputStringData;
use std::collections::HashMap;
use std::{
    fs::File,
    io::{stdin, BufRead, BufReader},
    path::PathBuf,
};
use string_stats::StringStats;

type GroupNumberStats = HashMap<String, NumberStats>;
type GroupStringStats = HashMap<String, (StringStats, NumberStats)>;

/// Grouped number stats on stream (count, min, max, mean, stddev).
/// Takes the last column of the provided data as the number value to analyze.
/// All preceding columns are interpreted as grouping data.
#[derive(Parser)]
struct Cli {
    /// input delimiter
    #[arg(short = 'd', long)]
    input_delimiter: char,

    /// Optional output delimiter, default to human readable table output
    #[arg(short = 'D', long)]
    output_delimiter: Option<char>,

    /// Optional number of decimals to round for output
    #[arg(short = 'r', long, default_value_t = 0)]
    decimals: usize,

    /// Count zeros as null, in addition to always counting non-numbers as null
    #[arg(short, long, default_value_t = false)]
    zero_as_null: bool,

    /// Interpret as strings, return stats about length and value
    /// Default is to interpret as numbers
    #[arg(short, long, default_value_t = false)]
    strings: bool,

    /// Optional cap on cardinality, set to zero to disable cardinality
    #[arg(short, long)]
    cardinality_cap: Option<usize>,

    /// Count empty strings as null, in addition to always countint non-numbers as null
    #[arg(short, long, default_value_t = false)]
    empty_as_null: bool,

    /// The path to the file to read, use - to read from stdin (must not be a tty)
    #[arg(default_value = "-")]
    file: PathBuf,
}

fn main() {
    let args = Cli::parse();
    let file = args.file;

    if args.strings {
        let group_string_stats = if file == PathBuf::from("-") {
            if stdin().is_terminal() {
                Cli::command().print_help().unwrap();
                ::std::process::exit(2);
            }
            group_string_stats_in_buf_reader(
                BufReader::new(stdin().lock()),
                args.input_delimiter,
                args.empty_as_null,
                args.cardinality_cap,
            )
        } else {
            group_string_stats_in_buf_reader(
                BufReader::new(File::open(&file).unwrap()),
                args.input_delimiter,
                args.empty_as_null,
                args.cardinality_cap,
            )
        };
        OutputStringData::new(
            group_string_stats,
            args.input_delimiter,
            args.output_delimiter,
            args.decimals,
            args.cardinality_cap,
        )
        .print();
    } else {
        let group_number_stats = if file == PathBuf::from("-") {
            if stdin().is_terminal() {
                Cli::command().print_help().unwrap();
                ::std::process::exit(2);
            }
            group_number_stats_in_buf_reader(
                BufReader::new(stdin().lock()),
                args.input_delimiter,
                args.zero_as_null,
            )
        } else {
            group_number_stats_in_buf_reader(
                BufReader::new(File::open(&file).unwrap()),
                args.input_delimiter,
                args.zero_as_null,
            )
        };
        OutputNumberData::new(
            group_number_stats,
            args.input_delimiter,
            args.output_delimiter,
            args.decimals,
        )
        .print();
    }
}

fn group_number_stats_in_buf_reader<R: BufRead>(
    buf_reader: R,
    delimiter: char,
    zero_as_null: bool,
) -> GroupNumberStats {
    let mut group_number_stats = GroupNumberStats::new();
    for line in buf_reader.lines() {
        let raw = line.unwrap();
        match raw.rsplit_once(delimiter) {
            Some((group, value)) => {
                let number_stats = group_number_stats
                    .entry(group.to_string())
                    .or_insert(NumberStats::new());
                match value.parse::<f64>() {
                    Ok(num) if zero_as_null && num == 0.0 => number_stats.add_null(),
                    Ok(num) => number_stats.add(num),
                    Err(_) => number_stats.add_null(),
                };
            }
            None => {
                group_number_stats
                    .entry("<INVALID>".to_string())
                    .and_modify(|number_stats| number_stats.add_null())
                    .or_insert(NumberStats::new());
            }
        }
    }
    group_number_stats
}

fn group_string_stats_in_buf_reader<R: BufRead>(
    buf_reader: R,
    delimiter: char,
    empty_as_null: bool,
    cardinality_cap: Option<usize>,
) -> GroupStringStats {
    let mut group_string_stats = GroupStringStats::new();
    for line in buf_reader.lines() {
        let raw = line.unwrap();
        match raw.rsplit_once(delimiter) {
            Some((group, value)) => {
                let (value_stats, length_stats) = group_string_stats
                    .entry(group.to_string())
                    .or_insert((StringStats::new(cardinality_cap), NumberStats::new()));

                if empty_as_null && value.is_empty() {
                    length_stats.add_null();
                    value_stats.add_null();
                } else {
                    length_stats.add(value.len() as f64);
                    value_stats.add(value.to_string());
                };
            }
            None => {
                group_string_stats
                    .entry("<INVALID>".to_string())
                    .and_modify(|(value_stats, _length_stats)| value_stats.add_null())
                    .or_insert((StringStats::new(cardinality_cap), NumberStats::new()));
            }
        }
    }
    group_string_stats
}
