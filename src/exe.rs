use crate::convtbl::TaiUtcTable;
use crate::{error::Error, DT_FMT};
use clap::{App, Arg, ArgMatches, Values};
use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
mod converter;
pub mod error;
pub mod execcode;
pub use converter::{main_convertion, Converter};
pub mod tai2utc;
pub mod tt2utc;
pub mod utc2tai;
pub mod utc2tt;

const TAI_UTC_TABLE_FILENAME: &str = "tai-utc.txt";
const TAI_UTC_TABLE: &str = include_str!("tai-utc.txt");

pub fn print_err(stderr: &mut impl Write, err: &dyn std::fmt::Display) {
    writeln!(stderr, "{}: {}", exe_name(), err).unwrap();
}

pub fn exe_name() -> String {
    return PathBuf::from(env::args().next().unwrap())
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap()
        .to_string();
}

pub fn load_tai_utc_table(
    table_file_path: Option<&PathBuf>,
    datetime_fmt: &str,
) -> Result<TaiUtcTable, Error> {
    match table_file_path {
        Some(table_file_path) => {
            let table_file = File::open(table_file_path)
                .map_err(|_| Error::TaiUtcTableIOError(table_file_path.clone()))?;
            let table_lines = BufReader::new(table_file)
                .lines()
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| Error::TaiUtcTableNotTextError(table_file_path.clone()))?;
            TaiUtcTable::from_lines(table_lines, datetime_fmt)
        }
        None => {
            let table_lines: Vec<_> = TAI_UTC_TABLE.split("\n").collect();
            TaiUtcTable::from_lines(table_lines, datetime_fmt)
        }
    }
}

/// Serve a method for output to stdout
///
/// # Arguments
/// * `params` - Parameters of execution
///
/// # Returns
/// A method for output to stdout. It requires arguments which it needs for output.
pub fn get_print_line(params: &Parameters) -> fn(&mut dyn Write, &str, &str) -> () {
    match params.io_pair_flg() {
        false => |out: &mut dyn Write, _: &str, o: &str| writeln!(out, "{}", o).unwrap(),
        true => |out: &mut dyn Write, i: &str, o: &str| writeln!(out, "{} {}", i, o).unwrap(),
    }
}

/// Command arguments of convdate
#[derive(Debug)]
pub struct Arguments<'a> {
    matches: ArgMatches<'a>,
    tai_utc_table_dt_fmt: Option<String>,
    dt_fmt: Option<String>,
    io_pair_flg: bool,
    tai_utc_table_path: Option<String>,
}

impl Arguments<'_> {
    pub fn new<'a>(
        app_name: &str,
        args: impl IntoIterator<Item = impl Into<OsString> + Clone>,
    ) -> Arguments<'a> {
        let app: App<'a, 'a> = App::new(app_name)
            .arg(
                Arg::with_name("tai_utc_table_dt_fmt")
                    .help("Format of datetime in TAI-UTC table file. If it is not specified, the environment variable 'TAI_UTC_TABLE_DT_FMT' is used. If both of them are not specified, the default value \"%Y-%m-%dT%H:%M:%S%.3f\" is used.")
                    .takes_value(true)
                    .long("tai-utc-table-dt-fmt"),
            )
            .arg(
                Arg::with_name("dt_fmt")
                    .help("Format of <datetime>. If it is not specified, the environment variable 'DT_FMT' is used. If both of them are not specified, the default value \"%Y-%m-%dT%H:%M:%S%.3f\" is used.")
                    .takes_value(true)
                    .long("dt-fmt"),
            )
            .arg(
                Arg::with_name("io_pair_flg")
                    .help("If it is specified, input datetime is also output to stdin.")
                    .short("H")
                    .long("io-pair"),
            )
            .arg(
                Arg::with_name("tai_utc_table_file")
                    .help("Filepath of TAI-UTC table file. If it is not specified, the environment variable 'TAI_UTC_TABLE' is used. If both of them are not specified, the default file ({binaries_directory}/tai-utc.txt) is used. If the default file also does not exist, use the built-in table in the program.")
                    .takes_value(true)
                    .long("tai-utc-table"),
            )
            .arg(
                Arg::with_name("datetime")
                    .help("datetime to convert. Instead of specifying it here, you can also enter it from the standard input.")
                    .multiple(true),
            );
        let matches: ArgMatches<'a> = app.get_matches_from(args);
        Arguments::<'a> {
            tai_utc_table_dt_fmt: matches
                .value_of("tai_utc_table_dt_fmt")
                .map(|s| s.to_string()),
            dt_fmt: matches.value_of("dt_fmt").map(|s| s.to_string()),
            io_pair_flg: matches.is_present("io_pair_flg"),
            tai_utc_table_path: matches
                .value_of("tai_utc_table_file")
                .map(|s| s.to_string()),
            matches: matches,
        }
    }

    pub fn get_dt_fmt(&self) -> Option<&str> {
        self.dt_fmt.as_ref().map(|s| s.as_str())
    }

    pub fn get_tai_utc_table_dt_fmt(&self) -> Option<&str> {
        self.tai_utc_table_dt_fmt.as_ref().map(|s| s.as_str())
    }

    pub fn get_tai_utc_table_path(&self) -> Option<&str> {
        self.tai_utc_table_path.as_ref().map(|s| s.as_str())
    }

    pub fn get_io_pair_flg(&self) -> bool {
        self.io_pair_flg
    }

    pub fn get_datetimes(&self) -> Option<Values> {
        return self.matches.values_of("datetime");
    }
}

/// Environment variables which convdate uses
#[derive(Debug)]
pub struct EnvValues {
    dt_fmt: Option<String>,
    tai_utc_table_dt_fmt: Option<String>,
    tai_utc_table_path: Option<String>,
}

impl EnvValues {
    pub fn new(iter: impl IntoIterator<Item = (impl ToString, impl ToString)>) -> EnvValues {
        let map = iter
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect::<HashMap<_, _>>();
        EnvValues {
            dt_fmt: map.get("DT_FMT").map(|s| s.to_string()),
            tai_utc_table_dt_fmt: map.get("TAI_UTC_TABLE_DT_FMT").map(|s| s.to_string()),
            tai_utc_table_path: map.get("TAI_UTC_TABLE").map(|s| s.to_string()),
        }
    }

    pub fn get_dt_fmt(&self) -> Option<&str> {
        self.dt_fmt.as_ref().map(|s| s.as_str())
    }

    pub fn get_tai_utc_table_dt_fmt(&self) -> Option<&str> {
        self.tai_utc_table_dt_fmt.as_ref().map(|s| s.as_str())
    }

    pub fn get_tai_utc_table_path(&self) -> Option<&str> {
        self.tai_utc_table_path.as_ref().map(|s| s.as_str())
    }
}

#[derive(Debug)]
pub struct Parameters<'a> {
    args: &'a Arguments<'a>,
    dt_fmt: &'a str,
    tai_utc_table_dt_fmt: &'a str,
    tai_utc_table_path: Option<PathBuf>,
    io_pair_flg: bool,
}

impl Parameters<'_> {
    pub fn new<'a>(args: &'a Arguments, env_vars: &'a EnvValues) -> Parameters<'a> {
        return Parameters {
            args,
            dt_fmt: Parameters::decide_dt_fmt(args, env_vars),
            tai_utc_table_dt_fmt: Parameters::decide_tai_utc_table_dt_fmt(args, env_vars),
            tai_utc_table_path: Parameters::decide_tai_utc_table_path(args, env_vars),
            io_pair_flg: args.io_pair_flg,
        };
    }

    pub fn get_datetimes(&self) -> Option<Values> {
        return self.args.matches.values_of("datetime");
    }

    pub fn get_dt_fmt(&self) -> &str {
        return &self.dt_fmt;
    }

    pub fn get_tai_utc_table_dt_fmt(&self) -> &str {
        return &self.tai_utc_table_dt_fmt;
    }

    fn decide_dt_fmt<'a>(args: &'a Arguments, env_vars: &'a EnvValues) -> &'a str {
        args.get_dt_fmt()
            .or_else(|| env_vars.get_dt_fmt())
            .unwrap_or(DT_FMT)
    }

    fn decide_tai_utc_table_dt_fmt<'a>(args: &'a Arguments, env_vars: &'a EnvValues) -> &'a str {
        args.get_tai_utc_table_dt_fmt()
            .or_else(|| env_vars.get_tai_utc_table_dt_fmt())
            .unwrap_or(DT_FMT)
    }

    pub fn io_pair_flg(&self) -> bool {
        return self.io_pair_flg;
    }

    pub fn get_tai_utc_table_path(&self) -> Option<&PathBuf> {
        return self.tai_utc_table_path.as_ref();
    }

    fn decide_tai_utc_table_path(args: &Arguments, env_vars: &EnvValues) -> Option<PathBuf> {
        // If it is specified as command args, use it.
        if let Some(path) = args.get_tai_utc_table_path() {
            return Some(PathBuf::from(path));
        }

        // If it is specified as environment variable, use it.
        if let Some(path) = env_vars.get_tai_utc_table_path() {
            return Some(PathBuf::from(path));
        }

        // If default file exists, use it.
        let mut exe_path = env::current_exe().unwrap();
        exe_path.pop();
        exe_path.push(TAI_UTC_TABLE_FILENAME);
        if exe_path.exists() {
            return Some(exe_path);
        }

        // use builtin default
        return None;
    }
}
