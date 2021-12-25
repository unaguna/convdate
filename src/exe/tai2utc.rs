use super::{Arguments, EnvValues, Parameters};
use crate::{exe, tai2utc};
use std::ffi::OsString;
use std::io::{BufRead, Write};

pub fn main_inner(
    args: impl IntoIterator<Item = impl Into<OsString> + Clone>,
    env_vars: impl IntoIterator<Item = (impl ToString, impl ToString)>,
    stdin: &mut impl BufRead,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> i32 {
    let args = Arguments::new("Converter from TAI to UTC", args);
    let env_vars = EnvValues::new(env_vars);

    // Analyze the arguments and the environment variables.
    let params = Parameters::new(&args, &env_vars);

    // load TAI-UTC table
    let tai_utc_table = exe::load_tai_utc_table(
        params.get_tai_utc_table_path(),
        params.get_tai_utc_table_dt_fmt(),
    );
    let utc_tai_table = match tai_utc_table {
        Ok(tai_utc_table) => From::from(&tai_utc_table),
        Err(e) => {
            exe::print_err(stderr, &e);
            return exe::EXIT_CODE_NG;
        }
    };

    // function for output to stdout
    let print_line = exe::get_print_line(&params);

    // Chooses input datetimes stream
    let dt_stream: Box<dyn Iterator<Item = Result<String, _>>> = match args.get_datetimes() {
        Some(datetimes) => Box::new(datetimes.map(|s| Ok(s.to_string()))),
        None => Box::new(stdin.lines()),
    };

    // calc UTC
    let mut someone_is_err = false;
    for in_tai in dt_stream {
        let in_tai = match in_tai {
            Ok(in_tai) => in_tai,
            Err(e) => {
                someone_is_err = true;
                exe::print_err(stderr, &e);

                // This error occurs when the input stream is invalid.
                // In other words, subsequent inputs are also likely to be abnormal,
                // so the process is terminated without `continue`.
                break;
            }
        };

        let utc = tai2utc(&in_tai, &utc_tai_table, params.get_dt_fmt());

        match utc {
            Err(e) => {
                someone_is_err = true;
                exe::print_err(stderr, &e)
            }
            Ok(utc) => print_line(stdout, &in_tai, &utc),
        }
    }

    return if someone_is_err {
        exe::EXIT_CODE_SOME_DT_NOT_CONVERTED
    } else {
        exe::EXIT_CODE_OK
    };
}

#[cfg(test)]
mod tests {
    use super::main_inner;
    use crate::{exe, testmod};
    use std::collections::HashMap;

    const EXE_NAME: &str = "utc2tt";

    /// Test regular case.
    #[test]
    fn test_simply() {
        let args = vec![
            EXE_NAME,
            "2015-07-01T00:00:34.000",
            "2015-07-01T00:00:35.001",
            "2015-07-01T00:00:36.002",
            "2015-07-01T00:00:37.003",
            "2015-07-01T00:00:38.004",
            "2017-01-01T00:00:35",
            "2017-01-01T00:00:36",
            "2017-01-01T00:00:37",
            "2017-01-01T00:00:38",
            "2017-01-01T00:00:39",
        ];
        let env_vars = HashMap::<String, String>::from([]);
        let stdin_buf = b"";
        let mut stdout_buf = Vec::<u8>::new();
        let mut stderr_buf = Vec::<u8>::new();

        // Run the target.
        let exec_code = main_inner(
            args,
            env_vars,
            &mut &stdin_buf[..],
            &mut stdout_buf,
            &mut stderr_buf,
        );

        assert_eq!(exec_code, 0);
        assert_eq!(
            String::from_utf8_lossy(&stdout_buf),
            "2015-06-30T23:59:59.000\n\
            2015-06-30T23:59:60.001\n\
            2015-07-01T00:00:00.002\n\
            2015-07-01T00:00:01.003\n\
            2015-07-01T00:00:02.004\n\
            2016-12-31T23:59:59.000\n\
            2016-12-31T23:59:60.000\n\
            2017-01-01T00:00:00.000\n\
            2017-01-01T00:00:01.000\n\
            2017-01-01T00:00:02.000\n"
        );
        assert_eq!(String::from_utf8_lossy(&stderr_buf), "");
    }

    /// Test error when input datetimes are illegal.
    #[test]
    fn test_input_dt_illegal_against_default_dt_fmt() {
        let test_dir = testmod::tmp_dir(Some("")).unwrap();
        let tai_utc_table_path = testmod::tmp_tai_utc_table(
            &test_dir,
            &vec![
                "2012-07-01T00:00:00 5",
                "2015-07-01T00:00:00 6",
                "2017-01-01T00:00:00 7",
            ],
        )
        .unwrap();

        let args = vec![
            EXE_NAME,
            "2015-07-01T00:00:04.000",
            "2015-07-01T00:00:05.001",
            "2010-07-01T00:00:06.002",
            "2015-07-01T00:00:07.003",
            "2015-07-01T00:00:08.004",
            "2017-01-0100:00:05",
            "2017-01-0100:00:06",
            "2017-01-01T00:00:07",
            "2017-01-01T00:00:08",
            "2017-01-01T00:00:09",
        ];
        let env_vars = HashMap::from([("TAI_UTC_TABLE", tai_utc_table_path.to_str().unwrap())]);
        let stdin_buf = b"";
        let mut stdout_buf = Vec::<u8>::new();
        let mut stderr_buf = Vec::<u8>::new();

        // Run the target.
        let exec_code = main_inner(
            args,
            env_vars,
            &mut &stdin_buf[..],
            &mut stdout_buf,
            &mut stderr_buf,
        );

        assert_eq!(exec_code, 2);
        assert_eq!(
            String::from_utf8_lossy(&stdout_buf),
            "2015-06-30T23:59:59.000\n\
            2015-06-30T23:59:60.001\n\
            2015-07-01T00:00:01.003\n\
            2015-07-01T00:00:02.004\n\
            2017-01-01T00:00:00.000\n\
            2017-01-01T00:00:01.000\n\
            2017-01-01T00:00:02.000\n"
        );
        assert_eq!(
            String::from_utf8_lossy(&stderr_buf),
            format!(
                "{}: {}\n{}: {}\n{}: {}\n",
                exe::exe_name(),
                "The datetime is too low: 2010-07-01 00:00:06.002",
                exe::exe_name(),
                "Cannot parse the datetime: 2017-01-0100:00:05",
                exe::exe_name(),
                "Cannot parse the datetime: 2017-01-0100:00:06"
            )
        );
    }

    /// Test error when TAI-UTC table data are illegal.
    #[test]
    fn test_tai_utc_table_illegal() {
        let test_dir = testmod::tmp_dir(Some("")).unwrap();
        let tai_utc_table_path = testmod::tmp_tai_utc_table(
            &test_dir,
            &vec![
                "2012-07-01T00:00:00 5",
                "2015-07-01T00:00:00 A",
                "2017-01-01T00:00:00 7",
            ],
        )
        .unwrap();

        let args = vec![
            EXE_NAME,
            "2015-07-01T00:00:04.000",
            "2015-07-01T00:00:05.001",
            "2015-07-01T00:00:06.002",
            "2015-07-01T00:00:07.003",
            "2015-07-01T00:00:08.004",
            "2017-01-01T00:00:05",
            "2017-01-01T00:00:06",
            "2017-01-01T00:00:07",
            "2017-01-01T00:00:08",
            "2017-01-01T00:00:09",
        ];
        let env_vars = HashMap::from([("TAI_UTC_TABLE", tai_utc_table_path.to_str().unwrap())]);
        let stdin_buf = b"";
        let mut stdout_buf = Vec::<u8>::new();
        let mut stderr_buf = Vec::<u8>::new();

        // Run the target.
        let exec_code = main_inner(
            args,
            env_vars,
            &mut &stdin_buf[..],
            &mut stdout_buf,
            &mut stderr_buf,
        );

        assert_eq!(exec_code, 1);
        assert_eq!(String::from_utf8_lossy(&stdout_buf), "");
        assert_eq!(
            String::from_utf8_lossy(&stderr_buf),
            format!(
                "{}: {}\n",
                exe::exe_name(),
                "Illegal definition of TAI-UTC difference: 2015-07-01T00:00:00 A"
            )
        );
    }

    /// Test error when datetimes in TAI-UTC table are illegal.
    #[test]
    fn test_tai_utc_table_dt_illegal_against_default_tai_utc_table_dt_fmt() {
        let test_dir = testmod::tmp_dir(Some("")).unwrap();
        let tai_utc_table_path = testmod::tmp_tai_utc_table(
            &test_dir,
            &vec![
                "2012-07-01T00:00:00 5",
                "2015-07-0100:00:00 6",
                "2017-01-01T00:00:00 7",
            ],
        )
        .unwrap();

        let args = vec![
            EXE_NAME,
            "2015-07-01T00:00:04.000",
            "2015-07-01T00:00:05.001",
            "2015-07-01T00:00:06.002",
            "2015-07-01T00:00:07.003",
            "2015-07-01T00:00:08.004",
            "2017-01-01T00:00:05",
            "2017-01-01T00:00:06",
            "2017-01-01T00:00:07",
            "2017-01-01T00:00:08",
            "2017-01-01T00:00:09",
        ];
        let env_vars = HashMap::from([("TAI_UTC_TABLE", tai_utc_table_path.to_str().unwrap())]);
        let stdin_buf = b"";
        let mut stdout_buf = Vec::<u8>::new();
        let mut stderr_buf = Vec::<u8>::new();

        // Run the target.
        let exec_code = main_inner(
            args,
            env_vars,
            &mut &stdin_buf[..],
            &mut stdout_buf,
            &mut stderr_buf,
        );

        assert_eq!(exec_code, 1);
        assert_eq!(String::from_utf8_lossy(&stdout_buf), "");
        assert_eq!(
            String::from_utf8_lossy(&stderr_buf),
            format!(
                "{}: {}\n",
                exe::exe_name(),
                "Illegal definition of TAI-UTC difference (datetime): 2015-07-0100:00:00"
            )
        );
    }

    /// Test error when an environment variable TAI_UTC_TABLE is a path which is not exists
    #[test]
    fn test_env_tai_utc_table_not_exist() {
        let tai_utc_table_path = "/tmp/dummy/not_exists.txt";

        let args = vec![
            EXE_NAME,
            "2015-07-01T00:00:04.000",
            "2015-07-01T00:00:05.001",
            "2015-07-01T00:00:06.002",
            "2015-07-01T00:00:07.003",
            "2015-07-01T00:00:08.004",
            "2017-01-01T00:00:05",
            "2017-01-01T00:00:06",
            "2017-01-01T00:00:07",
            "2017-01-01T00:00:08",
            "2017-01-01T00:00:09",
        ];
        let env_vars = HashMap::from([("TAI_UTC_TABLE", tai_utc_table_path)]);
        let stdin_buf = b"";
        let mut stdout_buf = Vec::<u8>::new();
        let mut stderr_buf = Vec::<u8>::new();

        // Run the target.
        let exec_code = main_inner(
            args,
            env_vars,
            &mut &stdin_buf[..],
            &mut stdout_buf,
            &mut stderr_buf,
        );

        assert_eq!(exec_code, 1);
        assert_eq!(String::from_utf8_lossy(&stdout_buf), "");
        assert_eq!(
            String::from_utf8_lossy(&stderr_buf),
            format!(
                "{}: {}\n",
                exe::exe_name(),
                "The TAI-UTC table file isn't available: /tmp/dummy/not_exists.txt"
            )
        );
    }

    /// Test an argument --tai-utc-table.
    #[test]
    fn test_arg_tai_utc_table() {
        let test_dir = testmod::tmp_dir(Some("")).unwrap();
        let tai_utc_table_path = testmod::tmp_tai_utc_table(
            &test_dir,
            &vec![
                "2012-07-01T00:00:00 5",
                "2015-07-01T00:00:00 6",
                "2017-01-01T00:00:00 7",
            ],
        )
        .unwrap();

        let args = vec![
            EXE_NAME,
            "2015-07-01T00:00:04",
            "2015-07-01T00:00:05",
            "2015-07-01T00:00:06",
            "2015-07-01T00:00:07",
            "2015-07-01T00:00:08",
            "2017-01-01T00:00:05",
            "2017-01-01T00:00:06",
            "2017-01-01T00:00:07",
            "2017-01-01T00:00:08",
            "2017-01-01T00:00:09",
            "--tai-utc-table",
            tai_utc_table_path.to_str().unwrap(),
        ];
        let env_vars: HashMap<&str, &str> = HashMap::from([]);
        let stdin_buf = b"";
        let mut stdout_buf = Vec::<u8>::new();
        let mut stderr_buf = Vec::<u8>::new();

        // Run the target.
        let exec_code = main_inner(
            args,
            env_vars,
            &mut &stdin_buf[..],
            &mut stdout_buf,
            &mut stderr_buf,
        );

        assert_eq!(exec_code, 0);
        assert_eq!(
            String::from_utf8_lossy(&stdout_buf),
            "2015-06-30T23:59:59.000\n\
            2015-06-30T23:59:60.000\n\
            2015-07-01T00:00:00.000\n\
            2015-07-01T00:00:01.000\n\
            2015-07-01T00:00:02.000\n\
            2016-12-31T23:59:59.000\n\
            2016-12-31T23:59:60.000\n\
            2017-01-01T00:00:00.000\n\
            2017-01-01T00:00:01.000\n\
            2017-01-01T00:00:02.000\n"
        );
        assert_eq!(String::from_utf8_lossy(&stderr_buf), "");
    }

    /// Test error when an argument --tai-utc-table is a path which is not exists
    #[test]
    fn test_arg_tai_utc_table_not_exist() {
        let tai_utc_table_path = "/tmp/dummy/not_exists.txt";

        let args = vec![
            EXE_NAME,
            "2015-07-01T00:00:04",
            "2015-07-01T00:00:05",
            "2015-07-01T00:00:06",
            "2015-07-01T00:00:07",
            "2015-07-01T00:00:08",
            "2017-01-01T00:00:05",
            "2017-01-01T00:00:06",
            "2017-01-01T00:00:07",
            "2017-01-01T00:00:08",
            "2017-01-01T00:00:09",
            "--tai-utc-table",
            tai_utc_table_path,
        ];
        let env_vars: HashMap<&str, &str> = HashMap::from([]);
        let stdin_buf = b"";
        let mut stdout_buf = Vec::<u8>::new();
        let mut stderr_buf = Vec::<u8>::new();

        // Run the target.
        let exec_code = main_inner(
            args,
            env_vars,
            &mut &stdin_buf[..],
            &mut stdout_buf,
            &mut stderr_buf,
        );

        assert_eq!(exec_code, 1);
        assert_eq!(String::from_utf8_lossy(&stdout_buf), "");
        assert_eq!(
            String::from_utf8_lossy(&stderr_buf),
            format!(
                "{}: {}\n",
                exe::exe_name(),
                "The TAI-UTC table file isn't available: /tmp/dummy/not_exists.txt"
            )
        );
    }

    /// Test an environment variable TAI_UTC_TABLE.
    #[test]
    fn test_env_tai_utc_table() {
        let test_dir = testmod::tmp_dir(Some("")).unwrap();
        let tai_utc_table_path = testmod::tmp_tai_utc_table(
            &test_dir,
            &vec![
                "2012-07-01T00:00:00 5",
                "2015-07-01T00:00:00 6",
                "2017-01-01T00:00:00 7",
            ],
        )
        .unwrap();

        let args = vec![
            EXE_NAME,
            "2015-07-01T00:00:04.000",
            "2015-07-01T00:00:05.001",
            "2015-07-01T00:00:06.002",
            "2015-07-01T00:00:07.003",
            "2015-07-01T00:00:08.004",
            "2017-01-01T00:00:05",
            "2017-01-01T00:00:06",
            "2017-01-01T00:00:07",
            "2017-01-01T00:00:08",
            "2017-01-01T00:00:09",
        ];
        let env_vars = HashMap::from([("TAI_UTC_TABLE", tai_utc_table_path.to_str().unwrap())]);
        let stdin_buf = b"";
        let mut stdout_buf = Vec::<u8>::new();
        let mut stderr_buf = Vec::<u8>::new();

        // Run the target.
        let exec_code = main_inner(
            args,
            env_vars,
            &mut &stdin_buf[..],
            &mut stdout_buf,
            &mut stderr_buf,
        );

        assert_eq!(exec_code, 0);
        assert_eq!(
            String::from_utf8_lossy(&stdout_buf),
            "2015-06-30T23:59:59.000\n\
            2015-06-30T23:59:60.001\n\
            2015-07-01T00:00:00.002\n\
            2015-07-01T00:00:01.003\n\
            2015-07-01T00:00:02.004\n\
            2016-12-31T23:59:59.000\n\
            2016-12-31T23:59:60.000\n\
            2017-01-01T00:00:00.000\n\
            2017-01-01T00:00:01.000\n\
            2017-01-01T00:00:02.000\n"
        );
        assert_eq!(String::from_utf8_lossy(&stderr_buf), "");
    }

    /// Test that an argument --tai-utc-table has a priority to an environment variable TAI_UTC_TABLE.
    #[test]
    fn test_arg_tai_utc_table_against_env() {
        let test_dir = testmod::tmp_dir(Some("")).unwrap();
        let tai_utc_table_path = testmod::tmp_tai_utc_table(
            &test_dir,
            &vec![
                "2012-07-01T00:00:00 5",
                "2015-07-01T00:00:00 6",
                "2017-01-01T00:00:00 7",
            ],
        )
        .unwrap();
        let dummy_tai_utc_table_path =
            testmod::tmp_text_file(&test_dir, "dummy_tai_utc_table.txt", &vec!["XXX"]).unwrap();

        let args = vec![
            EXE_NAME,
            "2015-07-01T00:00:04",
            "2015-07-01T00:00:05",
            "2015-07-01T00:00:06",
            "2015-07-01T00:00:07",
            "2015-07-01T00:00:08",
            "2017-01-01T00:00:05",
            "2017-01-01T00:00:06",
            "2017-01-01T00:00:07",
            "2017-01-01T00:00:08",
            "2017-01-01T00:00:09",
            "--tai-utc-table",
            tai_utc_table_path.to_str().unwrap(),
        ];
        let env_vars =
            HashMap::from([("TAI_UTC_TABLE", dummy_tai_utc_table_path.to_str().unwrap())]);
        let stdin_buf = b"";
        let mut stdout_buf = Vec::<u8>::new();
        let mut stderr_buf = Vec::<u8>::new();

        // Run the target.
        let exec_code = main_inner(
            args,
            env_vars,
            &mut &stdin_buf[..],
            &mut stdout_buf,
            &mut stderr_buf,
        );

        assert_eq!(exec_code, 0);
        assert_eq!(
            String::from_utf8_lossy(&stdout_buf),
            "2015-06-30T23:59:59.000\n\
            2015-06-30T23:59:60.000\n\
            2015-07-01T00:00:00.000\n\
            2015-07-01T00:00:01.000\n\
            2015-07-01T00:00:02.000\n\
            2016-12-31T23:59:59.000\n\
            2016-12-31T23:59:60.000\n\
            2017-01-01T00:00:00.000\n\
            2017-01-01T00:00:01.000\n\
            2017-01-01T00:00:02.000\n"
        );
        assert_eq!(String::from_utf8_lossy(&stderr_buf), "");
    }

    /// Test an argument --dt-fmt.
    #[test]
    fn test_arg_dt_fmt() {
        let test_dir = testmod::tmp_dir(Some("")).unwrap();
        let tai_utc_table_path = testmod::tmp_tai_utc_table(
            &test_dir,
            &vec![
                "2012-07-01T00:00:00 5",
                "2015-07-01T00:00:00 6",
                "2017-01-01T00:00:00 7",
            ],
        )
        .unwrap();

        let args = vec![
            EXE_NAME,
            "20150701000004",
            "20150701000005",
            "20150701000006",
            "20150701000007",
            "20150701000008",
            "20170101000005",
            "20170101000006",
            "20170101000007",
            "20170101000008",
            "20170101000009",
            "--dt-fmt",
            "%Y%m%d%H%M%S",
        ];
        let env_vars = HashMap::from([("TAI_UTC_TABLE", tai_utc_table_path.to_str().unwrap())]);
        let stdin_buf = b"";
        let mut stdout_buf = Vec::<u8>::new();
        let mut stderr_buf = Vec::<u8>::new();

        // Run the target.
        let exec_code = main_inner(
            args,
            env_vars,
            &mut &stdin_buf[..],
            &mut stdout_buf,
            &mut stderr_buf,
        );

        assert_eq!(exec_code, 0);
        assert_eq!(
            String::from_utf8_lossy(&stdout_buf),
            "20150630235959\n\
            20150630235960\n\
            20150701000000\n\
            20150701000001\n\
            20150701000002\n\
            20161231235959\n\
            20161231235960\n\
            20170101000000\n\
            20170101000001\n\
            20170101000002\n"
        );
        assert_eq!(String::from_utf8_lossy(&stderr_buf), "");
    }

    /// Test an environment variable DT_FMT.
    #[test]
    fn test_env_dt_fmt() {
        let test_dir = testmod::tmp_dir(Some("")).unwrap();
        let tai_utc_table_path = testmod::tmp_tai_utc_table(
            &test_dir,
            &vec![
                "2012-07-01T00:00:00 5",
                "2015-07-01T00:00:00 6",
                "2017-01-01T00:00:00 7",
            ],
        )
        .unwrap();

        let args = vec![
            EXE_NAME,
            "20150701000004",
            "20150701000005",
            "20150701000006",
            "20150701000007",
            "20150701000008",
            "20170101000005",
            "20170101000006",
            "20170101000007",
            "20170101000008",
            "20170101000009",
        ];
        let env_vars = HashMap::from([
            ("TAI_UTC_TABLE", tai_utc_table_path.to_str().unwrap()),
            ("DT_FMT", "%Y%m%d%H%M%S"),
        ]);
        let stdin_buf = b"";
        let mut stdout_buf = Vec::<u8>::new();
        let mut stderr_buf = Vec::<u8>::new();

        // Run the target.
        let exec_code = main_inner(
            args,
            env_vars,
            &mut &stdin_buf[..],
            &mut stdout_buf,
            &mut stderr_buf,
        );

        assert_eq!(exec_code, 0);
        assert_eq!(
            String::from_utf8_lossy(&stdout_buf),
            "20150630235959\n\
            20150630235960\n\
            20150701000000\n\
            20150701000001\n\
            20150701000002\n\
            20161231235959\n\
            20161231235960\n\
            20170101000000\n\
            20170101000001\n\
            20170101000002\n"
        );
        assert_eq!(String::from_utf8_lossy(&stderr_buf), "");
    }

    /// Test that an argument --dt-fmt has a priority to an environment variable DT_FMT.
    #[test]
    fn test_arg_dt_fmt_against_env() {
        let test_dir = testmod::tmp_dir(Some("")).unwrap();
        let tai_utc_table_path = testmod::tmp_tai_utc_table(
            &test_dir,
            &vec![
                "2012-07-01T00:00:00 5",
                "2015-07-01T00:00:00 6",
                "2017-01-01T00:00:00 7",
            ],
        )
        .unwrap();

        let args = vec![
            EXE_NAME,
            "2015/07/01-00:00:04",
            "2015/07/01-00:00:05",
            "2015/07/01-00:00:06",
            "2015/07/01-00:00:07",
            "2015/07/01-00:00:08",
            "2017/01/01-00:00:05",
            "2017/01/01-00:00:06",
            "2017/01/01-00:00:07",
            "2017/01/01-00:00:08",
            "2017/01/01-00:00:09",
            "--dt-fmt",
            "%Y/%m/%d-%H:%M:%S",
        ];
        let env_vars = HashMap::from([
            ("TAI_UTC_TABLE", tai_utc_table_path.to_str().unwrap()),
            ("DT_FMT", "%Y%m%d%H%M%S"),
        ]);
        let stdin_buf = b"";
        let mut stdout_buf = Vec::<u8>::new();
        let mut stderr_buf = Vec::<u8>::new();

        // Run the target.
        let exec_code = main_inner(
            args,
            env_vars,
            &mut &stdin_buf[..],
            &mut stdout_buf,
            &mut stderr_buf,
        );

        assert_eq!(exec_code, 0);
        assert_eq!(
            String::from_utf8_lossy(&stdout_buf),
            "2015/06/30-23:59:59\n\
            2015/06/30-23:59:60\n\
            2015/07/01-00:00:00\n\
            2015/07/01-00:00:01\n\
            2015/07/01-00:00:02\n\
            2016/12/31-23:59:59\n\
            2016/12/31-23:59:60\n\
            2017/01/01-00:00:00\n\
            2017/01/01-00:00:01\n\
            2017/01/01-00:00:02\n"
        );
        assert_eq!(String::from_utf8_lossy(&stderr_buf), "");
    }

    /// Test an argument --tai-utc-table-dt-fmt.
    #[test]
    fn test_arg_tai_utc_table_dt_fmt() {
        let test_dir = testmod::tmp_dir(Some("")).unwrap();
        let tai_utc_table_path = testmod::tmp_tai_utc_table(
            &test_dir,
            &vec![
                "20120701000000000 5",
                "20150701000000000 6",
                "20170101000000000 7",
            ],
        )
        .unwrap();

        let args = vec![
            EXE_NAME,
            "2015-07-01T00:00:04",
            "2015-07-01T00:00:05",
            "2015-07-01T00:00:06",
            "2015-07-01T00:00:07",
            "2015-07-01T00:00:08",
            "2017-01-01T00:00:05",
            "2017-01-01T00:00:06",
            "2017-01-01T00:00:07",
            "2017-01-01T00:00:08",
            "2017-01-01T00:00:09",
            "--tai-utc-table-dt-fmt",
            "%Y%m%d%H%M%S%3f",
        ];
        let env_vars = HashMap::from([("TAI_UTC_TABLE", tai_utc_table_path.to_str().unwrap())]);
        let stdin_buf = b"";
        let mut stdout_buf = Vec::<u8>::new();
        let mut stderr_buf = Vec::<u8>::new();

        // Run the target.
        let exec_code = main_inner(
            args,
            env_vars,
            &mut &stdin_buf[..],
            &mut stdout_buf,
            &mut stderr_buf,
        );

        assert_eq!(exec_code, 0);
        assert_eq!(
            String::from_utf8_lossy(&stdout_buf),
            "2015-06-30T23:59:59.000\n\
            2015-06-30T23:59:60.000\n\
            2015-07-01T00:00:00.000\n\
            2015-07-01T00:00:01.000\n\
            2015-07-01T00:00:02.000\n\
            2016-12-31T23:59:59.000\n\
            2016-12-31T23:59:60.000\n\
            2017-01-01T00:00:00.000\n\
            2017-01-01T00:00:01.000\n\
            2017-01-01T00:00:02.000\n"
        );
        assert_eq!(String::from_utf8_lossy(&stderr_buf), "");
    }

    /// Test an environment variable TAI_UTC_TABLE_DT_FMT.
    #[test]
    fn test_env_tai_utc_table_dt_fmt() {
        let test_dir = testmod::tmp_dir(Some("")).unwrap();
        let tai_utc_table_path = testmod::tmp_tai_utc_table(
            &test_dir,
            &vec![
                "20120701000000000 5",
                "20150701000000000 6",
                "20170101000000000 7",
            ],
        )
        .unwrap();

        let args = vec![
            EXE_NAME,
            "2015-07-01T00:00:04",
            "2015-07-01T00:00:05",
            "2015-07-01T00:00:06",
            "2015-07-01T00:00:07",
            "2015-07-01T00:00:08",
            "2017-01-01T00:00:05",
            "2017-01-01T00:00:06",
            "2017-01-01T00:00:07",
            "2017-01-01T00:00:08",
            "2017-01-01T00:00:09",
        ];
        let env_vars = HashMap::from([
            ("TAI_UTC_TABLE", tai_utc_table_path.to_str().unwrap()),
            ("TAI_UTC_TABLE_DT_FMT", "%Y%m%d%H%M%S%3f"),
        ]);
        let stdin_buf = b"";
        let mut stdout_buf = Vec::<u8>::new();
        let mut stderr_buf = Vec::<u8>::new();

        // Run the target.
        let exec_code = main_inner(
            args,
            env_vars,
            &mut &stdin_buf[..],
            &mut stdout_buf,
            &mut stderr_buf,
        );

        assert_eq!(exec_code, 0);
        assert_eq!(
            String::from_utf8_lossy(&stdout_buf),
            "2015-06-30T23:59:59.000\n\
            2015-06-30T23:59:60.000\n\
            2015-07-01T00:00:00.000\n\
            2015-07-01T00:00:01.000\n\
            2015-07-01T00:00:02.000\n\
            2016-12-31T23:59:59.000\n\
            2016-12-31T23:59:60.000\n\
            2017-01-01T00:00:00.000\n\
            2017-01-01T00:00:01.000\n\
            2017-01-01T00:00:02.000\n"
        );
        assert_eq!(String::from_utf8_lossy(&stderr_buf), "");
    }

    /// Test that an argument --tai-utc-table-dt-fmt has a priority to an environment variable TAI_UTC_TABLE_DT_FMT
    #[test]
    fn test_arg_tai_utc_table_dt_fmt_against_env() {
        let test_dir = testmod::tmp_dir(Some("")).unwrap();
        let tai_utc_table_path = testmod::tmp_tai_utc_table(
            &test_dir,
            &vec![
                "2012/07/01-00:00:00 5",
                "2015/07/01-00:00:00 6",
                "2017/01/01-00:00:00 7",
            ],
        )
        .unwrap();

        let args = vec![
            EXE_NAME,
            "2015-07-01T00:00:04",
            "2015-07-01T00:00:05",
            "2015-07-01T00:00:06",
            "2015-07-01T00:00:07",
            "2015-07-01T00:00:08",
            "2017-01-01T00:00:05",
            "2017-01-01T00:00:06",
            "2017-01-01T00:00:07",
            "2017-01-01T00:00:08",
            "2017-01-01T00:00:09",
            "--tai-utc-table-dt-fmt",
            "%Y/%m/%d-%H:%M:%S",
        ];
        let env_vars = HashMap::from([
            ("TAI_UTC_TABLE", tai_utc_table_path.to_str().unwrap()),
            ("TAI_UTC_TABLE_DT_FMT", "%Y%m%d%H%M%S%3f"),
        ]);
        let stdin_buf = b"";
        let mut stdout_buf = Vec::<u8>::new();
        let mut stderr_buf = Vec::<u8>::new();

        // Run the target.
        let exec_code = main_inner(
            args,
            env_vars,
            &mut &stdin_buf[..],
            &mut stdout_buf,
            &mut stderr_buf,
        );

        assert_eq!(exec_code, 0);
        assert_eq!(
            String::from_utf8_lossy(&stdout_buf),
            "2015-06-30T23:59:59.000\n\
            2015-06-30T23:59:60.000\n\
            2015-07-01T00:00:00.000\n\
            2015-07-01T00:00:01.000\n\
            2015-07-01T00:00:02.000\n\
            2016-12-31T23:59:59.000\n\
            2016-12-31T23:59:60.000\n\
            2017-01-01T00:00:00.000\n\
            2017-01-01T00:00:01.000\n\
            2017-01-01T00:00:02.000\n"
        );
        assert_eq!(String::from_utf8_lossy(&stderr_buf), "");
    }
}
