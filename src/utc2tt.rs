use crate::convtbl::TaiUtcTable;
use crate::error::Error;
use crate::{tai2tt_dt, utc2tai_dt};
use chrono::NaiveDateTime;

/// Convert datetime
/// from [UTC](https://en.wikipedia.org/wiki/Coordinated_Universal_Time)
/// to [TT](https://en.wikipedia.org/wiki/Terrestrial_Time).
///
/// This function takes leap seconds into account along the argument `tai_utc_table`.
///
/// # Arguments
/// * `datetime` - Datetime in UTC.
/// * `tai_utc_table` - The conversion table of TAI - UTC
/// * `dt_fmt` - [format](https://docs.rs/chrono/0.4.19/chrono/format/strftime/index.html) of `datetime`
///
/// # Returns
/// Returns the datetime in TT.
///
/// Returns [`Error`](crate::error::Error) if it fail to convert.
///
/// # Examples
/// ```
/// use convdate;
/// use convdate::convtbl::TaiUtcTable;
///
/// // Usually, lines read from the file are used as the argument of `from_lines`.
/// let tai_utc_table = TaiUtcTable::from_lines(vec!["2017-01-01T00:00:00 37"], "%Y-%m-%dT%H:%M:%S").unwrap();
///
/// let tt = convdate::utc2tt(
///     "2017-01-01T12:00:00.000",
///     &tai_utc_table,
///     "%Y-%m-%dT%H:%M:%S%.3f");
///
/// assert_eq!(tt, Ok("2017-01-01T12:01:09.184".to_string()));
/// ```
///
/// # See also
/// * [`utc2tt_dt`] - It is same as `utc2tt`, except that the argument and the result are [`NaiveDateTime`].
/// * [`utc2tt`](../utc2tt/index.html) (Binary crate) - The executable program which do same conversion.
pub fn utc2tt(datetime: &str, tai_utc_table: &TaiUtcTable, dt_fmt: &str) -> Result<String, Error> {
    let datetime = NaiveDateTime::parse_from_str(datetime, dt_fmt)
        .map_err(|_e| Error::DatetimeParseError(datetime.to_string()))?;
    let tai = utc2tt_dt(&datetime, tai_utc_table)?;
    Ok(tai.format(dt_fmt).to_string())
}

/// Convert datetime
/// from [UTC](https://en.wikipedia.org/wiki/Coordinated_Universal_Time)
/// to [TT](https://en.wikipedia.org/wiki/Terrestrial_Time).
///
/// This function takes leap seconds into account along the argument `tai_utc_table`.
///
/// # Arguments
/// * `datetime` - Datetime in UTC.
/// * `tai_utc_table` - The conversion table of TAI - UTC
///
/// # Returns
/// Returns the datetime in TT.
///
/// Returns [`Error`](crate::error::Error) if it fail to convert.
///
/// # Examples
/// ```
/// use convdate;
/// use convdate::convtbl::TaiUtcTable;
/// use chrono::NaiveDate;
///
/// // Usually, lines read from the file are used as the argument of `from_lines`.
/// let tai_utc_table = TaiUtcTable::from_lines(vec!["2017-01-01T00:00:00 37"], "%Y-%m-%dT%H:%M:%S").unwrap();
///
/// let tt = convdate::utc2tt_dt(
///     &NaiveDate::from_ymd(2017, 1, 1).and_hms(12, 0, 0),
///     &tai_utc_table);
///
/// assert_eq!(tt, Ok(NaiveDate::from_ymd(2017, 1, 1).and_hms_milli(12, 1, 9, 184)));
/// ```
///
/// # See also
/// * [`utc2tt`] - It is same as `utc2tt_dt`, except that the argument and the result are [`str`] and [`String`].
/// * [`utc2tt`](../utc2tt/index.html) (Binary crate) - The executable program which do same conversion.
pub fn utc2tt_dt(
    datetime: &NaiveDateTime,
    tai_utc_table: &TaiUtcTable,
) -> Result<NaiveDateTime, Error> {
    let tai = utc2tai_dt(datetime, tai_utc_table)?;
    Ok(tai2tt_dt(&tai))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::convtbl::DiffTaiUtc;
    use chrono::NaiveDate;
    use rstest::*;

    const DT_FMT: &str = "%Y-%m-%dT%H:%M:%S%.3f";

    #[rstest]
    // Error when the input datetime is too low.
    #[case("2015-06-30T23:59:60.999", None, Some(Error::DatetimeTooLowError("2015-06-30 23:59:60.999".to_string())))]
    // regular cases
    #[case("2017-01-02T11:22:33.000", Some("2017-01-02T11:23:42.184"), None)]
    #[case("2017-01-02T11:22:33.123", Some("2017-01-02T11:23:42.307"), None)]
    // うるう秒が挿入される瞬間のテスト
    #[case("2016-12-31T23:59:59.000", Some("2017-01-01T00:01:07.184"), None)]
    #[case("2016-12-31T23:59:60.000", Some("2017-01-01T00:01:08.184"), None)]
    #[case("2016-12-31T23:59:60.123", Some("2017-01-01T00:01:08.307"), None)]
    #[case("2017-01-01T00:00:00.000", Some("2017-01-01T00:01:09.184"), None)]
    // うるう秒が削除される瞬間のテスト
    #[case("2017-12-31T23:59:58.000", Some("2018-01-01T00:01:07.184"), None)]
    #[case("2017-12-31T23:59:58.123", Some("2018-01-01T00:01:07.307"), None)]
    #[case("2018-01-01T00:00:00.000", Some("2018-01-01T00:01:08.184"), None)]
    // うるう秒が2秒挿入される瞬間のテスト
    #[case("2018-12-31T23:59:59.000", Some("2019-01-01T00:01:07.184"), None)]
    #[case("2018-12-31T23:59:60.000", Some("2019-01-01T00:01:08.184"), None)]
    // #[case("2018-12-31T23:59:61.000", Some("2019-01-01T00:01:09.184"), None)]
    #[case("2019-01-01T00:00:00.000", Some("2019-01-01T00:01:10.184"), None)]
    // うるう秒が2秒削除される瞬間のテスト
    #[case("2019-12-31T23:59:57.000", Some("2020-01-01T00:01:07.184"), None)]
    #[case("2020-01-01T00:00:00.000", Some("2020-01-01T00:01:08.184"), None)]
    // Error when the input datetime is illegal format.
    #[case("2019-12-31 23:59:57.000", None, Some(Error::DatetimeParseError(utc.to_string())))]
    fn test_utc2tai(
        #[case] utc: &str,
        #[case] expected_ok: Option<&str>,
        #[case] expected_err: Option<Error>,
    ) {
        let expected = expected_ok
            .map(ToString::to_string)
            .ok_or_else(|| expected_err.unwrap());

        let tai_utc_table = vec![
            DiffTaiUtc {
                datetime: NaiveDate::from_ymd(2015, 7, 1).and_hms(0, 0, 0),
                diff_seconds: 36,
            },
            DiffTaiUtc {
                datetime: NaiveDate::from_ymd(2017, 1, 1).and_hms(0, 0, 0),
                diff_seconds: 37,
            },
            DiffTaiUtc {
                datetime: NaiveDate::from_ymd(2018, 1, 1).and_hms(0, 0, 0),
                diff_seconds: 36,
            },
            DiffTaiUtc {
                datetime: NaiveDate::from_ymd(2019, 1, 1).and_hms(0, 0, 0),
                diff_seconds: 38,
            },
            DiffTaiUtc {
                datetime: NaiveDate::from_ymd(2020, 1, 1).and_hms(0, 0, 0),
                diff_seconds: 36,
            },
        ];
        let tai = utc2tt(&utc, &tai_utc_table.into(), DT_FMT);

        assert_eq!(tai, expected);
    }

    #[rstest]
    #[case(
        "2017-01-02T11:22:33.000",
        "%Y-%m-%dT%H:%M:%S%.3f",
        Some("2017-01-02T11:23:42.184"),
        None
    )]
    #[case(
        "2017-01-02T11:22:33.123",
        "%Y-%m-%dT%H:%M:%S%.3f",
        Some("2017-01-02T11:23:42.307"),
        None
    )]
    #[case(
        "2017-01-02T11:22:33",
        "%Y-%m-%dT%H:%M:%S%.3f",
        Some("2017-01-02T11:23:42.184"),
        None
    )]
    #[case(
        "2017-01-02T11:22:33",
        "%Y-%m-%dT%H:%M:%S",
        Some("2017-01-02T11:23:42"),
        None
    )]
    #[case(
        "2017-01-02 11:22:33",
        "%Y-%m-%d %H:%M:%S",
        Some("2017-01-02 11:23:42"),
        None
    )]
    #[case(
        "2017-01-02T11:22:33",
        "%Y-%m-%d %H:%M:%S",
        None,
        Some(Error::DatetimeParseError(utc.to_string()))
    )]
    fn test_utc2tai_arg_dt_fmt(
        #[case] utc: &str,
        #[case] dt_fmt: &str,
        #[case] expected_ok: Option<&str>,
        #[case] expected_err: Option<Error>,
    ) {
        let expected = expected_ok
            .map(ToString::to_string)
            .ok_or_else(|| expected_err.unwrap());

        let tai_utc_table = vec![DiffTaiUtc {
            datetime: NaiveDate::from_ymd(2017, 1, 1).and_hms(0, 0, 0),
            diff_seconds: 37,
        }];
        let tai = utc2tt(&utc, &tai_utc_table.into(), dt_fmt);

        assert_eq!(tai, expected);
    }
}
