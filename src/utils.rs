//! Utility functions

use esp_idf_svc::systime::EspSystemTime;
use log::*;
use std::borrow::Cow;

/// Generate a pseudo-random number using system time
pub fn rand() -> u32 {
    let nanos = EspSystemTime::now(&EspSystemTime {}).subsec_nanos();
    let result = nanos / 65537;
    debug!(
        "Generated random number: {} (from nanos: {})",
        result, nanos
    );
    result
}

/// Convert a number to its ordinal form (1st, 2nd, 3rd, etc.)
pub fn nth(n: u32) -> Cow<'static, str> {
    let result = match n {
        smaller @ (0..=13) => Cow::Borrowed(match smaller {
            0 => "zeroth",
            1 => "first",
            2 => "second",
            3 => "third",
            4 => "fourth",
            5 => "fifth",
            6 => "sixth",
            7 => "seventh",
            8 => "eighth",
            9 => "ninth",
            10 => "10th",
            11 => "11th",
            12 => "12th",
            13 => "13th",
            _ => unreachable!(),
        }),
        larger => Cow::Owned(match larger % 10 {
            1 => format!("{larger}st"),
            2 => format!("{larger}nd"),
            3 => format!("{larger}rd"),
            _ => format!("{larger}th"),
        }),
    };
    debug!("Converted {} to ordinal: {}", n, result);
    result
}

#[cfg(test)]
mod tests {
   

    #[test]
    fn test_nth_small_numbers() {
        assert_eq!(nth(1), "first");
        assert_eq!(nth(2), "second");
        assert_eq!(nth(3), "third");
    }

    #[test]
    fn test_nth_larger_numbers() {
        assert_eq!(nth(21), "21st");
        assert_eq!(nth(22), "22nd");
        assert_eq!(nth(23), "23rd");
        assert_eq!(nth(24), "24th");
    }
}

