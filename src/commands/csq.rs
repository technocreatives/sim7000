use crate::{Error, SerialReadTimeout};

use super::{AtCommand, AtDecode, AtExecute, Decoder};

pub struct Csq;

impl AtCommand for Csq {
    const COMMAND: &'static str = "AT+CSQ";
}

impl AtExecute for Csq {
    type Output = SignalDiagnostics;
}

#[derive(Debug, Clone)]
pub struct SignalDiagnostics {
    pub signal_strength: Option<f32>,
    pub signal_quality: Option<f32>,
}

impl AtDecode for SignalDiagnostics {
    fn decode<B: SerialReadTimeout>(
        decoder: &mut Decoder<B>,
        timeout_ms: u32,
    ) -> Result<Self, Error<B::SerialError>> {
        decoder.expect_str("+CSQ: ", timeout_ms)?;

        let rssi = match decoder.decode_scalar(timeout_ms)? {
            0 => Some(-115),
            1 => Some(-111),
            lookup if lookup <= 31 => Some(-110 + (lookup - 2) * 2),
            99 => None,
            _ => return Err(crate::Error::DecodingFailed),
        };
        let signal_strength = rssi.map(|rssi| {
            // normalize rssi to 0, then percent can be calculated
            let normalized_rssi = rssi + 115;
            100.0 * (normalized_rssi as f32 / 63f32)
        });
        decoder.expect_str(",", timeout_ms)?;

        let bit_error_rate = match decoder.decode_scalar(timeout_ms)? {
            0 => Some(0.14f32),
            1 => Some(0.28f32),
            2 => Some(0.57f32),
            3 => Some(1.13f32),
            4 => Some(2.26f32),
            5 => Some(4.53f32),
            6 => Some(9.05f32),
            7 => Some(18.10f32),
            99 => {
                if rssi.is_none() {
                    None
                } else {
                    Some(0.0f32)
                }
            }
            _ => return Err(crate::Error::DecodingFailed),
        };

        let signal_quality = bit_error_rate.map(|error_rate| 100f32 - error_rate);

        decoder.end_line();

        decoder.expect_str("OK", timeout_ms)?;

        Ok(SignalDiagnostics {
            signal_strength,
            signal_quality,
        })
    }
}
