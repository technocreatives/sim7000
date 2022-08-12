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
        satellites_in_view: u32,
        satellites_used: u32,
        signal_noise_ratio: u32,
    },
}

impl ATParseLine for GnssReport {
    fn from_line(line: &str) -> Result<Self, ATParseErr> {
        let (message, rest) = line.split_once(": ").ok_or(ATParseErr)?;

        if message != "+UGNSINF" {
            return Err(ATParseErr);
        }

        // NOTE: +UGNSINF differs *slightlly from +CGNSINF
        //
        // +UGNSINF: <GNSS run status>,<Fix status>,<UTC date & Time>,
        // <Latitude>,<Longitude>,<MSL Altitude>,<Speed Over Ground>,
        // <Course Over Ground>,<Fix Mode>,<Reserved1>,<HDOP>,
        // <PDOP>,<VDOP>,<Reserved2>,<Satellites in View>,
        // <Satellites Used>,<Reserved3>,<C/N0 max>,<HPA>,<VPA>
        //
        //
        // +CGNSINF: <GNSS run status>,<Fix status>,<UTC date & Time>,
        // <Latitude>,<Longitude>,<MSL Altitude>,<Speed Over Ground>,
        // <Course Over Ground>,<Fix Mode>,<Reserved1>,<HDOP>,
        // <PDOP>,<VDOP>,<Reserved2>,<GNSS Satellites in View>,
        // <GNSS Satellites Used>,<GLONASS Satellites Used>,<Reserved3>,
        // <C/N0 max>,<HPA>,<VPA>
        let [run_status, fix_status, _utc_datetime, latitude, longitude, msl_altitude, speed_over_groud, course_over_ground, _fix_mode, _reserved1, hdop, pdop, vdop, _reserved2, satellites_in_view, satellites_used, _reserved3, c_n0_max, _hpa, _vpa] =
            collect_array(rest.split(',')).ok_or(ATParseErr)?;

        if run_status != "1" {
            return Ok(GnssReport::NotEnabled);
        }

        if fix_status != "1" {
            return Ok(GnssReport::NoFix {
                sat_gps_view: satellites_in_view.parse().ok(),
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
            satellites_in_view: satellites_in_view.parse()?,
            satellites_used: satellites_used.parse()?,
            signal_noise_ratio: c_n0_max.parse()?,
        })
    }
}
