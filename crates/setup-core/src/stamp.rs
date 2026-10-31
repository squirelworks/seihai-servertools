//! Dates for backup names and change comments, in UTC, with no dependency.

use std::time::{SystemTime, UNIX_EPOCH};

/// A UTC date and time to the second.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Stamp {
    pub year: i64,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

impl Stamp {
    /// Now, in UTC.
    pub fn now() -> Self {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        Self::from_unix(secs)
    }

    /// From seconds since 1970-01-01 00:00 UTC.
    pub fn from_unix(secs: i64) -> Self {
        let days = secs.div_euclid(86_400);
        let rem = secs.rem_euclid(86_400) as u32;
        // Civil-from-days (Howard Hinnant's public-domain algorithm).
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z.rem_euclid(146_097);
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let year = yoe + era * 400 + i64::from(month <= 2);
        Stamp {
            year,
            month,
            day,
            hour: rem / 3600,
            minute: rem / 60 % 60,
            second: rem % 60,
        }
    }

    /// `YYYY-MM-DD`, for change comments.
    pub fn date(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }

    /// `YYYYMMDD-HHMMSS`, for backup file names.
    pub fn compact(&self) -> String {
        format!(
            "{:04}{:02}{:02}-{:02}{:02}{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_dates() {
        assert_eq!(Stamp::from_unix(0).compact(), "19700101-000000");
        // 2026-10-31 19:07:11 UTC
        assert_eq!(Stamp::from_unix(1_793_473_631).compact(), "20261031-190711");
        // Leap day
        assert_eq!(Stamp::from_unix(951_782_400).date(), "2000-02-29");
    }
}
