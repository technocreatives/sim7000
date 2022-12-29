mod command;
mod context;
pub mod power;

use embassy_time::{with_timeout, Duration, TimeoutError, Timer};

use crate::{
    at_command::{
        at, ate, cbatchk, ccid,
        cedrxs::{self, AcTType, EDRXSetting},
        cfgri::{self, RiPinMode},
        cgmr, cgnspwr, cgnsurc, cgreg, cifsrex, ciicr, cipmux, cipshut, cipstart,
        cmee::{self, CMEErrorMode},
        cmnb::{self, NbMode},
        cnmp, cops, cpsi, csclk, csq, cstt,
        ifc::{self, FlowControl},
        ipr::{self, BaudRate},
        unsolicited::{ConnectionMessage, RegistrationStatus},
        At, AtRequest, ConnectMode, NetworkMode,
    },
    gnss::Gnss,
    log,
    pump::{DropPump, RawIoPump, RxPump, TxPump},
    read::ModemReader,
    tcp::{ConnectError, TcpStream},
    voltage::VoltageWarner,
    BuildIo, Error, ModemPower, PowerState,
};
pub use command::{CommandRunner, CommandRunnerGuard, RawAtCommand};
pub use context::*;

use self::{command::ExpectResponse, power::PowerSignalBroadcaster};

pub struct Uninitialized;
pub struct Disabled;
pub struct Enabled;
pub struct Sleeping;

pub struct Modem<'c, P> {
    context: &'c ModemContext,
    power_signal: PowerSignalBroadcaster<'c>,
    commands: CommandRunner<'c>,
    power: P,
}

impl<'c, P: ModemPower> Modem<'c, P> {
    pub async fn new<I: BuildIo>(
        io: I,
        power: P,
        context: &'c ModemContext,
    ) -> Result<
        (
            Modem<'c, P>,
            RawIoPump<'c, I>,
            TxPump<'c>,
            RxPump<'c>,
            DropPump<'c>,
        ),
        Error,
    > {
        let modem = Modem {
            commands: context.commands(),
            power_signal: context.power_signal.publisher(),
            context,
            power,
        };

        let io_pump = RawIoPump {
            io,
            rx: context.rx_pipe.writer(),
            tx: context.tx_pipe.reader(),
            power_state: PowerState::Off,
            power_signal: context.power_signal.subscribe(),
        };

        let rx_pump = RxPump {
            reader: ModemReader::new(context.rx_pipe.reader()),
            generic_response: context.generic_response.sender(),
            registration_events: &context.registration_events,
            tcp: &context.tcp,
            gnss: context.gnss_slot.peek(),
            voltage_warning: context.voltage_slot.peek(),
        };

        let tx_pump = TxPump {
            writer: context.tx_pipe.writer(),
            commands: context.commands.receiver(),
        };

        let drop_pump = DropPump {
            context,
            power_signal: context.power_signal.subscribe(),
            power_state: PowerState::Off,
        };

        Ok((modem, io_pump, tx_pump, rx_pump, drop_pump))
    }

    pub async fn init(&mut self) -> Result<(), Error> {
        self.deactivate().await;
        self.power.enable().await;
        self.power_signal.broadcast(PowerState::On).await;

        let commands = self.commands.lock().await;

        let set_flow_control = ifc::SetFlowControl {
            dce_by_dte: FlowControl::Hardware,
            dte_by_dce: FlowControl::Hardware,
        };

        // Turn on hardware flow control, the modem does not save this state on reboot.
        // We need to set it as fast as possible to avoid dropping bytes.
        for _ in 0..5 {
            if let Ok(Ok(_)) = with_timeout(Duration::from_millis(2000), async {
                commands.run(set_flow_control).await
            })
            .await
            {
                break;
            }
        }

        // Modem has been known to get stuck in an unresponsive state until we jiggle it by
        // enabling echo. This is fine.
        for _ in 0..5 {
            if let Ok(Ok(_)) = with_timeout(
                Duration::from_millis(1000),
                commands.run(ate::SetEcho(true)),
            )
            .await
            {
                break;
            }
        }

        commands.run(csclk::SetSlowClock(true)).await?;
        commands.run(At).await?;
        commands.run(ipr::SetBaudRate(BaudRate::Hz115200)).await?;
        commands.run(set_flow_control).await?;
        commands
            .run(cmee::ConfigureCMEErrors(CMEErrorMode::Numeric))
            .await?;
        commands.run(cnmp::SetNetworkMode(NetworkMode::Lte)).await?;
        commands.run(cmnb::SetNbMode(NbMode::CatM)).await?;
        commands.run(cfgri::ConfigureRiPin(RiPinMode::On)).await?;
        commands.run(cbatchk::EnableVBatCheck(true)).await?;

        let configure_edrx = cedrxs::ConfigureEDRX {
            n: EDRXSetting::Enable,
            act_type: AcTType::CatM,
            requested_edrx_value: 0b0000,
        };

        for _ in 0..5 {
            match commands.run(configure_edrx).await {
                Ok(_) => break,
                _ => Timer::after(Duration::from_millis(200)).await,
            }
        }
        commands.run(configure_edrx).await?;

        drop(commands);
        self.deactivate().await;
        Ok(())
    }

    pub async fn activate(&mut self) -> Result<(), Error> {
        self.power_signal.broadcast(PowerState::On).await;
        self.power.enable().await;
        let set_flow_control = ifc::SetFlowControl {
            dce_by_dte: FlowControl::Hardware,
            dte_by_dce: FlowControl::Hardware,
        };

        let commands = self.commands.lock().await;

        for _ in 0..5 {
            if let Ok(Ok(_)) = with_timeout(Duration::from_millis(2000), async {
                commands.run(set_flow_control).await
            })
            .await
            {
                break;
            }
        }
        commands.run(ate::SetEcho(false)).await?;
        commands
            .run(cgreg::ConfigureRegistrationUrc::EnableRegLocation)
            .await?;

        self.wait_for_registration(&commands).await?;

        commands.run(cipmux::EnableMultiIpConnection(true)).await?;
        commands.run(cipshut::ShutConnections).await?;

        self.authenticate(&commands).await?;
        Ok(())
    }

    pub async fn deactivate(&mut self) {
        self.power_signal.broadcast(PowerState::Off).await;
        self.context.tcp.disconnect_all().await;

        self.power.disable().await;
    }

    async fn wait_for_registration(&self, commands: &CommandRunnerGuard<'_>) -> Result<(), Error> {
        loop {
            if with_timeout(Duration::from_millis(2000), async {
                commands.run(cgreg::GetRegistrationStatus).await
            })
            .await
            .is_err()
            {
                continue;
            }

            match self.context.registration_events.wait().await {
                RegistrationStatus::RegisteredHome | RegistrationStatus::RegisteredRoaming => break,
                _ => Timer::after(Duration::from_millis(200)).await,
            }
        }

        Ok(())
    }

    async fn authenticate(&self, commands: &CommandRunnerGuard<'_>) -> Result<(), Error> {
        commands
            .run(cstt::StartTask {
                apn: "iot.1nce.net".into(),
                username: "".into(),
                password: "".into(),
            })
            .await?;

        commands.run(ciicr::StartGprs).await?;

        let (_ip, _) = commands.run(cifsrex::GetLocalIpExt).await?;

        Ok(())
    }

    pub async fn connect_tcp(
        &mut self,
        host: &str,
        port: u16,
    ) -> Result<TcpStream<'c>, ConnectError> {
        let tcp_context = self.context.tcp.claim().unwrap();

        self.commands
            .lock()
            .await
            .run(cipstart::Connect {
                mode: ConnectMode::Tcp,
                number: tcp_context.ordinal(),
                destination: host.try_into().map_err(|_| Error::BufferOverflow)?,
                port,
            })
            .await?;

        // Wait for a response.
        // Based on testing, a connection will timeout after ~120 seconds, so we add our own
        // timeout to this step to prevent us from waiting forever if the modem died.
        for _ in 0..21 {
            match with_timeout(Duration::from_secs(6), tcp_context.events().recv()).await {
                Err(TimeoutError) => {
                    // Make sure the modem is still responding to commands.
                    self.commands.lock().await.run(at::At).await?;
                }

                Ok(ConnectionMessage::Connected) => {
                    return Ok(TcpStream::new(
                        tcp_context,
                        &self.context.drop_channel,
                        self.context.commands(),
                    ));
                }

                Ok(ConnectionMessage::ConnectionFailed) => return Err(ConnectError::ConnectFailed),

                // This should never happen, since we guard against connections already being used.
                Ok(msg) => return Err(ConnectError::Unexpected(msg)),
            }
        }

        // The modem never got back to us, it probably died.
        Err(ConnectError::Other(Error::Timeout))
    }

    pub async fn claim_gnss(&mut self) -> Result<Option<Gnss<'c>>, Error> {
        let Some(reports) = self.context.gnss_slot.claim() else {
            return Ok(None);
        };

        self.commands
            .lock()
            .await
            .run(cgnspwr::SetGnssPower(true))
            .await?;

        self.commands
            .lock()
            .await
            .run(cgnsurc::ConfigureGnssUrc {
                period: 4, // TODO
            })
            .await?;

        Ok(Some(Gnss::new(
            reports,
            self.context.power_signal.subscribe(),
            &self.context.drop_channel,
        )))
    }

    pub async fn claim_voltage_warner(&mut self) -> Option<VoltageWarner<'c>> {
        VoltageWarner::take(&self.context.voltage_slot)
    }

    /// Run a single AT command on the modem. Use with care.
    pub async fn run_command<C, Response>(&self, command: C) -> Result<Response, Error>
    where
        C: AtRequest<Response = Response>,
        Response: ExpectResponse,
    {
        self.commands.lock().await.run(command).await
    }

    pub async fn query_system_info(&mut self) -> Result<cpsi::SystemInfo, Error> {
        let (info, _) = self.commands.lock().await.run(cpsi::GetSystemInfo).await?;
        Ok(info)
    }

    pub async fn query_signal(&mut self) -> Result<csq::SignalQuality, Error> {
        self.run_command(csq::GetSignalQuality)
            .await
            .map(|(response, _)| response)
    }

    pub async fn query_operator_info(&mut self) -> Result<cops::OperatorInfo, Error> {
        self.run_command(cops::GetOperatorInfo)
            .await
            .map(|(response, _)| response)
    }

    pub async fn query_iccid(&mut self) -> Result<ccid::Iccid, Error> {
        self.run_command(ccid::ShowIccid)
            .await
            .map(|(response, _)| response)
    }

    pub async fn query_firmware_version(&mut self) -> Result<cgmr::FwVersion, Error> {
        self.run_command(cgmr::GetFwVersion)
            .await
            .map(|(response, _)| response)
    }

    pub async fn sleep(&mut self) {
        self.power_signal.broadcast(PowerState::Sleeping).await;
        self.power.sleep().await;
    }

    pub async fn wake(&mut self) {
        self.power.wake().await;
        self.power_signal.broadcast(PowerState::On).await;
    }
}
