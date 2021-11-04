use crate::{Error, SerialReadTimeout};

use super::{AtCommand, AtDecode, AtExecute, Decoder};
use embedded_time::duration::Milliseconds;

pub struct Cgnsinf;

impl AtCommand for Cgnsinf {
    const COMMAND: &'static str = "AT+CGNSINF";
}

impl AtExecute for Cgnsinf {
    type Output = GnssResponse;
}

#[derive(Debug, Copy, Clone)]
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
        let hdop = results.next().and_then(|v| v.parse::<f32>().ok())?;
        let pdop = results.next().and_then(|v| v.parse::<f32>().ok())?;
        let vdop = results.next().and_then(|v| v.parse::<f32>().ok())?;
        let _reserved2 = results.next()?;
        let sat_gps_view = results.next().and_then(|v| v.parse::<u32>().ok())?;
        let _sat_gnss_used = results.next()?;
        let _sat_glonass_view = results.next()?;

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
        })
    }
}

impl AtDecode for GnssResponse {
    fn decode<B: SerialReadTimeout>(
        decoder: &mut Decoder<B>,
        timeout: Milliseconds,
    ) -> Result<Self, Error<B::SerialError>> {
        decoder.expect_str("+CGNSINF: ", timeout)?;

        let result = GnssResponse::decode_cgnsinf(decoder.remainder_str(timeout)?)
            .ok_or(crate::Error::DecodingFailed)?;

        decoder.end_line();
        decoder.expect_empty(timeout)?;
        decoder.end_line();
        decoder.expect_str("OK", timeout)?;

        Ok(result)
    }
}
