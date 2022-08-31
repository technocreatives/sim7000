use core::str::FromStr;

use crate::at_command::{ATParseErr, ATParseLine};
use crate::util::collect_array;

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum GnssReport {
    NotEnabled,
    NoFix {
        sat_gps_view: Option<u32>,
    },
    Fix {
        latitude: f32,
        longitude: f32,
        altitude: f32,
        hdop: f32,
        pdop: f32,
        vdop: f32,
        speed_over_ground: f32,
        course_over_ground: f32,
        sat_gps_in_view: u32,
        sat_gnss_used: u32,
        sat_glonass_used: u32,
        signal_noise_ratio: u32,
    },
}

impl ATParseLine for GnssReport {
    fn from_line(line: &str) -> Result<Self, ATParseErr> {
        let (message, rest) = line.split_once(": ").ok_or("Missing ': '")?;

        if message != "+UGNSINF" {
            return Err("Missing +UGNSINF prefix".into());
        }

        let [run_status, fix_status, _utc_datetime, latitude, longitude, msl_altitude, speed_over_groud, course_over_ground, _fix_mode, _reserved1, hdop, pdop, vdop, _reserved2, sat_gps_in_view, sat_gnss_used, sat_glonass_used, _reserved3, c_n0_max, _hpa, _vpa] =
            collect_array(rest.split(',')).ok_or("Missing ',' separators")?;

        if run_status != "1" {
            return Ok(GnssReport::NotEnabled);
        }

        if fix_status != "1" {
            return Ok(GnssReport::NoFix {
                sat_gps_view: sat_gps_in_view.parse().ok(),
            });
        }

        /// Try to parse a string to a value, returning the default if the string is empty
        fn parse_optional<T: FromStr + Default>(s: &str) -> Result<T, <T as FromStr>::Err> {
            s.parse()
                .or_else(|e| s.is_empty().then(T::default).ok_or(e))
        }

        Ok(GnssReport::Fix {
            latitude: latitude.parse()?,
            longitude: longitude.parse()?,
            altitude: msl_altitude.parse()?,

            // The docs are unclear on what fields are optional, so just assume everything except
            // the core values are.
            speed_over_ground: parse_optional(speed_over_groud)?,
            course_over_ground: parse_optional(course_over_ground)?,
            hdop: parse_optional(hdop)?,
            pdop: parse_optional(pdop)?,
            vdop: parse_optional(vdop)?,
            signal_noise_ratio: parse_optional(c_n0_max)?,

            // The docs contradicts itself on what these values are and what they are called
            // TODO: Make sure these are correct.
            sat_gps_in_view: parse_optional(sat_gps_in_view)?,
            sat_gnss_used: parse_optional(sat_gnss_used)?,
            sat_glonass_used: parse_optional(sat_glonass_used)?,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse() {
        let gnss_str = "+UGNSINF: 1,1,20171103022632.000,31.222067,121.354368,34.700,0.00,0.0,1,,1.1,1.4,0.9,,21,6,,,45,,";
        let gnss = GnssReport::from_line(gnss_str).expect("Parse GnssReport");

        let expected = GnssReport::Fix {
            latitude: 31.222067,
            longitude: 121.354368,
            altitude: 34.7,
            hdop: 1.1,
            pdop: 1.4,
            vdop: 0.9,
            speed_over_ground: 0.0,
            course_over_ground: 0.0,
            sat_gps_in_view: 21,
            sat_gnss_used: 6,
            sat_glonass_used: 0,
            signal_noise_ratio: 45,
        };

        assert_eq!(expected, gnss);
    }

    #[test]
    fn test_missing_dop() {
        let gnss_str =
            "+UGNSINF: 1,1,20220126140944.000,57.715185,11.973960,44.600,0.00,214.5,1,,1.4,,,,29,5,,,52,,";
        let gnss = GnssReport::from_line(gnss_str).expect("Parse GnssReport");

        let expected = GnssReport::Fix {
            latitude: 57.715185,
            longitude: 11.973960,
            altitude: 44.6,
            hdop: 1.4,
            pdop: 0.0,
            vdop: 0.0,
            speed_over_ground: 0.0,
            course_over_ground: 214.5,
            sat_gps_in_view: 29,
            sat_gnss_used: 5,
            sat_glonass_used: 0,
            signal_noise_ratio: 52,
        };

        assert_eq!(expected, gnss);
    }
}
