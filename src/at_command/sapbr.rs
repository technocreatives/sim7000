use core::fmt::Write;
use heapless::String;

use super::{AtRequest, GenericOk};

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum CmdType {
    CloseBearer = 0,
    OpenBearer = 1,
    QueryBearer = 2,
    SetBearerParameters = 3,
    GetBearerParameters = 4,
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ConParamType {
    Apn = 0,
    User = 1,
    Pwd = 2,
}

/// AT+SAPBR=...
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct BearerSettings {
    pub cmd_type: CmdType,
    pub con_param_type: ConParamType,
    pub apn: String<63>,
}

impl AtRequest for BearerSettings {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        let con_param_type = match self.con_param_type {
            ConParamType::Apn => "APN",
            ConParamType::User => "USER",
            ConParamType::Pwd => "PWD,",
        };

        let mut buf = String::new();

        match self.cmd_type {
            CmdType::OpenBearer => write!(buf, "AT+SAPBR={},1\r", self.cmd_type as u8).unwrap(),
            CmdType::SetBearerParameters => write!(
                buf,
                "AT+SAPBR={},1,{:?},{:?}\r",
                self.cmd_type as u8, con_param_type, self.apn
            )
            .unwrap(),
            _ => todo!(),
        }

        buf
    }
}
