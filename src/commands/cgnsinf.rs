use crate::{Error, SerialReadTimeout};

use super::{AtCommand, AtDecode, AtExecute, Decoder};

pub struct Cgnsinf;

impl AtCommand for Cgnsinf {
    const COMMAND: &'static str = "AT+CGNSINF";
}

impl AtExecute for Cgnsinf {
    type Output = GnssResponse;
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum GnssResponse {
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
        sat_gps_view: u32,
        sat_gnss_used: u32,
        sat_glonass_used: u32,
        signal_noise_ratio: u32,
    },
}

impl GnssResponse {
    pub fn is_fix(&self) -> bool {
        matches!(self, GnssResponse::Fix { .. })
    }

    fn decode_cgnsinf(params: &str) -> Option<GnssResponse> {
        let mut results = params.split(',');

        if results.next().and_then(|v| v.parse::<u32>().ok())? != 1 {
            return Some(GnssResponse::NotEnabled);
        }

        if results.next().and_then(|v| v.parse::<u32>().ok())? != 1 {
            // 1,0,,,,,,,0,,,,,,1,,,,30,,"
            let sat_gps_view = results.nth(12).and_then(|v| v.parse::<u32>().ok());
            return Some(GnssResponse::NoFix { sat_gps_view });
        }

        let _timestamp = results.next()?;
        let latitude = results.next().and_then(|v| v.parse::<f32>().ok())?;
        let longitude = results.next().and_then(|v| v.parse::<f32>().ok())?;
        let altitude = results.next().and_then(|v| v.parse::<f32>().ok())?;
        let speed_over_ground = results.next().and_then(|v| v.parse::<f32>().ok())?;
        let course_over_ground = results.next().and_then(|v| v.parse::<f32>().ok())?;
        let _fix_mode = results.next()?;
        let _reserved1 = results.next()?;
        let hdop = results
            .next()
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(0.0);
        let pdop = results
            .next()
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(0.0);
        let vdop = results
            .next()
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(0.0);
        let _reserved2 = results.next()?;
        let sat_gps_view = results.next().and_then(|v| v.parse::<u32>().ok())?;
        let sat_gnss_used = results.next().and_then(|v| v.parse::<u32>().ok())?;
        let sat_glonass_used = results
            .next()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0);
        let _reserved3 = results.next()?;
        let signal_noise_ratio = results.next().and_then(|v| v.parse::<u32>().ok())?;

        Some(GnssResponse::Fix {
            latitude,
            longitude,
            altitude,
            hdop,
            pdop,
            vdop,
            speed_over_ground,
            course_over_ground,
            sat_gps_view,
            sat_gnss_used,
            sat_glonass_used,
            signal_noise_ratio,
        })
    }
}

impl AtDecode for GnssResponse {
    fn decode<B: SerialReadTimeout>(
        decoder: &mut Decoder<B>,
        timeout_ms: u32,
    ) -> Result<Self, Error<B::SerialError>> {
        decoder.expect_str("+CGNSINF: ", timeout_ms)?;

        let result = GnssResponse::decode_cgnsinf(decoder.remainder_str(timeout_ms)?)
            .ok_or(crate::Error::DecodingFailed)?;

        decoder.end_line();
        decoder.expect_str("OK", timeout_ms)?;

        Ok(result)
    }
}

#[cfg(test)]
mod test {
    use super::GnssResponse;

    #[test]
    fn test_fix_gnss_parse() {
        let gnss_str = "1,1,20171103022632.000,31.222067,121.354368,34.700,0.00,0.0,1,,1.1,1.4,0.9,,21,6,,,45,,";
        let gnss = GnssResponse::decode_cgnsinf(gnss_str).unwrap();

        let expected = GnssResponse::Fix {
            latitude: 31.222067,
            longitude: 121.354368,
            altitude: 34.7,
            hdop: 1.1,
            pdop: 1.4,
            vdop: 0.9,
            speed_over_ground: 0.0,
            course_over_ground: 0.0,
            sat_gps_view: 21,
            sat_gnss_used: 6,
            sat_glonass_used: 0,
            signal_noise_ratio: 45,
        };

        assert_eq!(expected, gnss);
    }

    #[test]
    fn test_missing_dop() {
        let gnss_str =
            "1,1,20220126140944.000,57.715185,11.973960,44.600,0.00,214.5,1,,1.4,,,,29,5,,,52,,";
        let gnss = GnssResponse::decode_cgnsinf(gnss_str).unwrap();

        let expected = GnssResponse::Fix {
            latitude: 57.715185,
            longitude: 11.973960,
            altitude: 44.6,
            hdop: 1.4,
            pdop: 0.0,
            vdop: 0.0,
            speed_over_ground: 0.0,
            course_over_ground: 214.5,
            sat_gps_view: 29,
            sat_gnss_used: 5,
            sat_glonass_used: 0,
            signal_noise_ratio: 52,
        };

        assert_eq!(expected, gnss);
    }
}
