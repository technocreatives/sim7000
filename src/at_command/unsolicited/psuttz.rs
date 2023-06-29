use crate::{
    at_command::{unsolicited::Dst, AtParseErr, AtParseLine},
    collect_array,
};
use chrono::{DateTime, FixedOffset, NaiveDate};

/// Refresh network time and timezone
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Psuttz {
    #[cfg_attr(feature = "defmt", defmt(Debug2Format))]
    datetime: DateTime<FixedOffset>,

    /// Daylight savings time
    dst: Dst,
}

impl AtParseLine for Psuttz {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        let (message, rest) = line.split_once(": ").ok_or("Missing ': '")?;

        if message != "*PSUTTZ" {
            return Err("Missing *PSUTTZ prefix".into());
        }

        let [ymd, hms, timezone, dst] = collect_array(rest.splitn(7, ',')).ok_or("Missing ','")?;

        let ymd = ymd.trim_matches('"');
        let hms = hms.trim_matches('"');
        let timezone = timezone.trim_matches('"');

        let [year, month, day] = collect_array(ymd.splitn(3, '/')).ok_or("Missing '/'")?;
        let [hour, minute, second] = collect_array(hms.splitn(3, ':')).ok_or("Missing ':'")?;

        let tz_offset = if let Some(timezone) = timezone.strip_prefix('-') {
            -timezone.parse::<i32>()?
        } else if let Some(timezone) = timezone.strip_prefix('+') {
            timezone.parse::<i32>()?
        } else {
            timezone.parse::<i32>()?
        };

        let mut year = year.parse()?;
        if year < 2000 {
            year += 2000;
        }

        const HOUR: i32 = 3600;
        let tz_offset = FixedOffset::east_opt(tz_offset * HOUR).ok_or("Invalid tz offset")?;
        let datetime = NaiveDate::from_ymd_opt(year, month.parse()?, day.parse()?)
            .ok_or("Invalid date")?
            .and_hms_opt(hour.parse()?, minute.parse()?, second.parse()?)
            .ok_or("Invalid time")?
            .and_local_timezone(tz_offset)
            .latest()
            .ok_or("Invalid time-tz combo")?;

        #[cfg(feature = "defmt")]
        defmt::warn!("unimplemented: {:?}", defmt::Debug2Format(&datetime));

        Ok(Psuttz {
            datetime,
            dst: Dst::try_from_u8(dst.parse()?)?,
        })

    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse() {
        // No, I don't know why the example string has 3 quotes in it.
        // Stop asking.
        let s = "*PSUTTZ: 23/06/29,10:58:46\",\"+08\",1";
        let parsed = Psuttz::from_line(s).expect("Parse Psuttz");

        let expected = Psuttz {
            datetime: NaiveDate::from_ymd_opt(2023, 06, 29)
                .unwrap()
                .and_hms_opt(10, 58, 46)
                .unwrap()
                .and_local_timezone(FixedOffset::east_opt(8 * 3600).unwrap())
                .latest()
                .unwrap(),
            dst: Dst::_1hour,
        };
        assert_eq!(expected, parsed);

        let s = "*PSUTTZ: 23/06/29,10:58:46\",\"-02\",1";
        let parsed = Psuttz::from_line(s).expect("Parse Psuttz");

        let expected = Psuttz {
            datetime: NaiveDate::from_ymd_opt(2023, 06, 29)
                .unwrap()
                .and_hms_opt(10, 58, 46)
                .unwrap()
                .and_local_timezone(FixedOffset::west_opt(2 * 3600).unwrap())
                .latest()
                .unwrap(),
            dst: Dst::_1hour,
        };
        assert_eq!(expected, parsed);
    }
}
