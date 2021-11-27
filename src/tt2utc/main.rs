use conv_date::{error::Error, exe, tai2utc, tt2tai};

fn main() {
    // Analize the arguments
    let args = exe::Arguments::new("Converter from TT to UTC");

    // load leap list
    let leaps =
        exe::load_leaps(&args.get_leaps_path(), args.get_leaps_dt_fmt()).unwrap_or_else(|e| {
            exe::print_err(&e);
            std::process::exit(exe::EXIT_CODE_NG)
        });

    let print_line = match args.io_pair_flg() {
        false => |_: &str, o: &str| println!("{}", o),
        true => |i: &str, o: &str| println!("{} {}", i, o),
    };

    // calc UTC
    let mut someone_is_err = false;
    for in_tt in args.get_datetimes() {
        let utc = tt2tai(in_tt, args.get_dt_fmt())
            .and_then(|tai| tai2utc(&tai, &leaps, args.get_dt_fmt()));

        match utc {
            Err(Error::DatetimeTooLowError(_)) => {
                // 多段階で変換を行う場合、中間の日時文字列がエラーメッセージに使われている場合があるため、入力された日時文字列に置き換える。
                someone_is_err = true;
                exe::print_err(&Error::DatetimeTooLowError(in_tt.to_string()));
            }
            Err(e) => {
                someone_is_err = true;
                exe::print_err(&e)
            }
            Ok(utc) => print_line(in_tt, &utc),
        }
    }

    std::process::exit(if someone_is_err {
        exe::EXIT_CODE_SOME_DT_NOT_CONVERTED
    } else {
        exe::EXIT_CODE_OK
    });
}
