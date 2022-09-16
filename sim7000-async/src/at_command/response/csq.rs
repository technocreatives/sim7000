use crate::at_command::{ATParseErr, ATParseLine};

use super::{ATResponse, ResponseCode};

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SignalQuality {
    /// Signal strength percenage
    pub signal_strength: Option<f32>,

    /// Bit-Error Rate percentage
    pub signal_quality: Option<f32>,
}

impl ATParseLine for SignalQuality {
    fn from_line(line: &str) -> Result<Self, ATParseErr> {
        let line = line.strip_prefix("+CSQ: ").ok_or("Missing '+CSG: '")?;
        let (rssi, ber) = line.split_once(',').ok_or("Missing ','")?;
        let rssi: u8 = rssi.parse()?;
        let ber: u8 = ber.parse()?;

        let rssi: Option<i32> = match rssi {
            0 => Some(-115),
            1 => Some(-111),
            i @ 2..=31 => Some(-110 + (i as i32 - 2) * 2),
            99 => None,
            _ => return Err("Invalid RSSI value".into()),
        };

        let signal_strength = rssi.map(|rssi| {
            // normalize rssi to 0, then percent can be calculated
            let normalized_rssi = rssi + 115;
            100.0 * (normalized_rssi as f32 / 63f32)
        });

        let signal_quality = match ber {
            0 => Some(0.14f32),
            1 => Some(0.28f32),
            2 => Some(0.57f32),
            3 => Some(1.13f32),
            4 => Some(2.26f32),
            5 => Some(4.53f32),
            6 => Some(9.05f32),
            7 => Some(18.10f32),
            99 => None,
            _ => return Err("Invalid BER value".into()),
        };

        Ok(SignalQuality {
            signal_strength,
            signal_quality,
        })
    }
}

impl ATResponse for SignalQuality {
    fn from_generic(code: ResponseCode) -> Result<Self, ResponseCode> {
        match code {
            ResponseCode::SignalQuality(sq) => Ok(sq),
            _ => Err(code),
        }
    }
}
