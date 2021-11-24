use crate::{error::Error, normalize_leap, LeapUtc};
use anyhow::Result;
use chrono::{DateTime, Duration, NaiveDateTime, TimeZone, Utc};

/// Pick the leap object to use for calc tai from the datetime.
///
/// # Arguments
///
/// * `datetime` - A datetime to convert to tai
/// * `leaps` - A list of leap objects
fn pick_dominant_leap<'a>(datetime: &DateTime<Utc>, leaps: &'a [LeapUtc]) -> Result<&'a LeapUtc> {
    // 線形探索
    let mut prev_leap: Option<&LeapUtc> = None;
    for leap in leaps.iter() {
        if datetime < &leap.datetime {
            break;
        }
        prev_leap = Some(leap);
    }
    return match prev_leap {
        Some(leap) => Ok(leap),
        None => Err(Error::DatetimeTooLowError(datetime.to_string()))?,
    };
}

pub fn utc2tai(datetime: &str, leaps: &[LeapUtc], dt_fmt: &str) -> Result<String> {
    let datetime = Utc.datetime_from_str(datetime, dt_fmt)?;
    let tai = utc2tai_dt(&datetime, leaps)?;
    Ok(tai.format(dt_fmt).to_string())
}

fn utc2tai_dt(datetime: &DateTime<Utc>, leaps: &[LeapUtc]) -> Result<NaiveDateTime> {
    let datetime_nm = normalize_leap(datetime);

    return pick_dominant_leap(datetime, leaps)
        .map(|leap| datetime_nm + Duration::seconds(leap.diff_seconds));
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use rstest::*;

    const DT_FMT: &str = "%Y-%m-%dT%H:%M:%S%.3f";

    #[rstest]
    #[case("2017-01-02T11:22:33.000", "2017-01-02T11:23:10.000")]
    #[case("2017-01-02T11:22:33.123", "2017-01-02T11:23:10.123")]
    // うるう秒が挿入される瞬間のテスト
    #[case("2016-12-31T23:59:59.000", "2017-01-01T00:00:35.000")]
    #[case("2016-12-31T23:59:60.000", "2017-01-01T00:00:36.000")]
    #[case("2016-12-31T23:59:60.123", "2017-01-01T00:00:36.123")]
    #[case("2017-01-01T00:00:00.000", "2017-01-01T00:00:37.000")]
    // うるう秒が削除される瞬間のテスト
    #[case("2017-12-31T23:59:58.000", "2018-01-01T00:00:35.000")]
    #[case("2017-12-31T23:59:58.123", "2018-01-01T00:00:35.123")]
    #[case("2018-01-01T00:00:00.000", "2018-01-01T00:00:36.000")]
    // うるう秒が2秒挿入される瞬間のテスト
    #[case("2018-12-31T23:59:59.000", "2019-01-01T00:00:35.000")]
    #[case("2018-12-31T23:59:60.000", "2019-01-01T00:00:36.000")]
    // #[case("2018-12-31T23:59:61.000", "2019-01-01T00:00:37.000")]
    #[case("2019-01-01T00:00:00.000", "2019-01-01T00:00:38.000")]
    // うるう秒が2秒削除される瞬間のテスト
    #[case("2019-12-31T23:59:57.000", "2020-01-01T00:00:35.000")]
    #[case("2020-01-01T00:00:00.000", "2020-01-01T00:00:36.000")]
    fn test_utc2tai(#[case] utc: &str, #[case] expected_tai: &str) {
        let leaps = vec![
            LeapUtc {
                datetime: Utc.ymd(2015, 7, 1).and_hms(0, 0, 0),
                diff_seconds: 36,
            },
            LeapUtc {
                datetime: Utc.ymd(2017, 1, 1).and_hms(0, 0, 0),
                diff_seconds: 37,
            },
            LeapUtc {
                datetime: Utc.ymd(2018, 1, 1).and_hms(0, 0, 0),
                diff_seconds: 36,
            },
            LeapUtc {
                datetime: Utc.ymd(2019, 1, 1).and_hms(0, 0, 0),
                diff_seconds: 38,
            },
            LeapUtc {
                datetime: Utc.ymd(2020, 1, 1).and_hms(0, 0, 0),
                diff_seconds: 36,
            },
        ];
        let tai = utc2tai(&utc, &leaps, DT_FMT).unwrap();

        assert_eq!(tai, expected_tai);
    }
}
