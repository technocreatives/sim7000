use crate::modem::at_command::{ATParseErr, ATParseLine};
use crate::util::collect_array;

#[derive(Debug)]
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

// TODO: unit tests. example:
// UGNSINF: 1,1,20220815113233.000,57.715366,11.973866,120.500,0.00,0.0,1,,0.9,1.1,0.7,,20,8,5,,49,,
impl ATParseLine for GnssReport {
    fn from_line(line: &str) -> Result<Self, ATParseErr> {
        let (message, rest) = line.split_once(": ").ok_or(ATParseErr)?;

        if message != "+UGNSINF" {
            return Err(ATParseErr);
        }

        let [run_status, fix_status, _utc_datetime, latitude, longitude, msl_altitude, speed_over_groud, course_over_ground, _fix_mode, _reserved1, hdop, pdop, vdop, _reserved2, sat_gps_in_view, sat_gnss_used, sat_glonass_used, _reserved3, c_n0_max, _hpa, _vpa] =
            collect_array(rest.split(',')).ok_or(ATParseErr)?;

        if run_status != "1" {
            return Ok(GnssReport::NotEnabled);
        }

        if fix_status != "1" {
            return Ok(GnssReport::NoFix {
                sat_gps_view: sat_gps_in_view.parse().ok(),
            });
        }

        Ok(GnssReport::Fix {
            latitude: latitude.parse()?,
            longitude: longitude.parse()?,
            altitude: msl_altitude.parse()?,
            speed_over_ground: speed_over_groud.parse()?,
            course_over_ground: course_over_ground.parse()?,
            hdop: hdop.parse()?,
            pdop: pdop.parse()?,
            vdop: vdop.parse()?,
            signal_noise_ratio: c_n0_max.parse()?,

            // The docs contradicts itself on what these values are and what they are called
            // TODO: Make sure these are correct.
            sat_gps_in_view: sat_gps_in_view.parse()?,
            sat_gnss_used: sat_gnss_used.parse()?,
            sat_glonass_used: sat_glonass_used.parse()?,
        })
    }
}
