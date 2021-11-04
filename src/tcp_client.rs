use super::commands::{self, AtWrite, ConnectionResult};
use crate::{AtModem, Error};
use embedded_time::duration::Milliseconds;
use log::*;

pub struct TcpClient {
    read_timeout: Milliseconds,
    write_timeout: Milliseconds,
}

impl Default for TcpClient {
    fn default() -> Self {
        Self {
            read_timeout: Milliseconds::new(2000),
            write_timeout: Milliseconds::new(5000),
        }
    }
}

impl TcpClient {
    pub fn set_read_timeout(&mut self, timeout: Milliseconds) {
        self.read_timeout = timeout;
    }

    pub fn read_timeout(&self) -> Option<Milliseconds> {
        Some(self.read_timeout)
    }

    pub fn set_write_timeout(&mut self, timeout: Milliseconds) {
        self.write_timeout = timeout;
    }

    pub fn write_timeout(&self) -> Option<Milliseconds> {
        Some(self.write_timeout)
    }

    pub fn connect<T>(
        &mut self,
        modem: &mut T,
        host: &'static str,
        port: u16,
        timeout: Option<Milliseconds>,
    ) -> Result<(), Error<T::SerialError>>
    where
        T: AtModem,
    {
        self.disconnect(modem, timeout)?;
        let result = commands::Cipstart
            .write(
                commands::TcpConnectionParams {
                    mode: "TCP",
                    host,
                    port,
                },
                modem,
                timeout.unwrap_or(self.write_timeout),
            )?;

        if result == ConnectionResult::Failure {
            return Err(Error::ConnectFailed);
        }
        Ok(())
    }

    pub fn disconnect<T>(
        &mut self,
        modem: &mut T,
        timeout: Option<Milliseconds>,
    ) -> Result<(), Error<T::SerialError>>
    where
        T: AtModem,
    {
        let cmd = commands::Cipshut;
        modem
            .execute(cmd, timeout.unwrap_or(self.write_timeout))?;
        Ok(())
    }

    pub fn send<T>(&mut self, modem: &mut T, data: &[u8]) -> Result<(), Error<T::SerialError>>
    where
        T: AtModem,
    {
        let cmd = commands::Cipsend;
        if let Err(e) = AtModem::write(modem, cmd, data, self.write_timeout) {
            // // If the send command fails, still write the data to be sent into the buffer, to
            // // prevent the modem getting into a state where it expects data outside of this function.
            // // This is fixed properly by the modem not having power failures, and better modem command logic
            // modem.write(data).ok();
            return Err(e);
        }
        Ok(())
    }

    pub fn receive<T>(&mut self, modem: &mut T, data: &mut [u8]) -> Result<usize, Error<T::SerialError>>
    where
        T: AtModem,
    {
        let mut offset = 0usize;
        loop {
            let data_len = data.len();
            let bytes_left = data_len - offset;
            if bytes_left == 0 {
                return Ok(offset);
            }
            offset += self.try_receive(modem, &mut data[offset..data_len])?;
        }
    }

    pub fn try_receive<T>(&mut self, modem: &mut T, data: &mut [u8]) -> Result<usize, Error<T::SerialError>>
    where
        T: AtModem,
    {
        let bytes_left = data.len() as u16;
        if bytes_left == 0 {
            return Ok(0);
        }

        let response = AtModem::write(
            modem,
            commands::Ciprxget,
            commands::NetworkReceiveMode::GetBytes(bytes_left),
            self.read_timeout,
        )?;

        if let Some(bytes) = response.bytes {
            data[..bytes.len()].copy_from_slice(&bytes);
            return Ok(bytes.len());
        }

        Ok(0)
    }

    pub fn is_connected<T: AtModem>(&self, modem: &mut T) -> bool {
        let cmd = commands::Cipstatus;

        let result = matches!(
            modem.execute(cmd, self.write_timeout),
            Ok(commands::ConnectionState::ConnectOk)
        );
        info!("is_connected: {}", result);
        result
    }
}
