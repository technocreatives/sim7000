mod command;
mod context;
pub mod power;

use embassy_time::{with_timeout, Duration, Timer};
use futures::{select_biased, FutureExt};

use crate::{
    at_command::{
        ate, cbatchk, ccid,
        cedrxs::{self, AcTType, EDRXSetting},
        cfgri::{self, RiPinMode},
        cgmr, cgnapn, cgnspwr, cgnsurc, cgreg, cifsrex, ciicr, cipmux, cipshut,
        cmee::{self, CMEErrorMode},
        cmnb::{self, NbMode},
        cnmp, cops, cpsi, csclk, csq, cstt,
        ifc::{self, FlowControl},
        ipr::{self, BaudRate},
        unsolicited::RegistrationStatus,
        At, AtRequest, NetworkMode,
    },
    gnss::Gnss,
    log,
    pump::{DropPump, RawIoPump, RxPump, TxPump},
    read::ModemReader,
    tcp::{ConnectError, TcpStream},
    voltage::VoltageWarner,
    BuildIo, Error, ModemPower, PowerState,
};
pub use command::{CommandRunner, CommandRunnerGuard, RawAtCommand, AT_DEFAULT_TIMEOUT};
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
    apn: Option<heapless::String<63>>,
    ap_username: &'static str,
    ap_password: &'static str,
}

const MODEM_POWER_TIMEOUT: Duration = Duration::from_secs(30);

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
            apn: None,
            ap_username: "",
            ap_password: "",
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
        log::info!("initializing modem");
        self.deactivate().await;
        with_timeout(MODEM_POWER_TIMEOUT, self.power.enable()).await?;
        self.power_signal.broadcast(PowerState::On);

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

        log::info!("modem successfully initialized, turning it back off...");
        self.deactivate().await;

        Ok(())
    }

    pub fn set_apn(&mut self, apn: heapless::String<63>) {
        self.apn = Some(apn);
    }

    pub fn set_ap_username(&mut self, ap_username: &'static str) {
        self.ap_username = ap_username;
    }

    pub fn set_ap_password(&mut self, ap_password: &'static str) {
        self.ap_password = ap_password;
    }

    pub async fn activate(&mut self) -> Result<(), Error> {
        log::info!("activating modem");
        self.power_signal.broadcast(PowerState::On);
        with_timeout(MODEM_POWER_TIMEOUT, self.power.enable()).await?;
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

        let apn = match &self.apn {
            Some(apn) => self.apn.insert(apn.clone()),
            None => {
                log::debug!("no default APN set, checking network for suggested APN.");
                let (network_apn, _) = commands.run(cgnapn::GetNetworkApn).await?;
                let Some(apn) = network_apn.apn else {
                    log::error!("no APN set");
                    return Err(Error::NoApn);
                };
                self.apn.insert(apn)
            }
        };

        log::info!("authenticating with apn {:?}", apn);

        commands
            .run(cstt::StartTask {
                apn: apn.clone(),
                username: self.ap_username.into(),
                password: self.ap_password.into(),
            })
            .await?;

        commands.run(ciicr::StartGprs).await?;

        let (_ip, _) = commands.run(cifsrex::GetLocalIpExt).await?;

        log::info!("modem successfully activated");
        Ok(())
    }

    pub async fn deactivate(&mut self) {
        self.power_signal.broadcast(PowerState::Off);
        self.context.tcp.disconnect_all().await;

        if with_timeout(MODEM_POWER_TIMEOUT, self.power.disable())
            .await
            .is_err()
        {
            log::warn!("timeout while powering off the modem");
        }
    }

    async fn wait_for_registration(&self, commands: &CommandRunnerGuard<'_>) -> Result<(), Error> {
        let wait_for_registration = async move {
            loop {
                commands.run(cgreg::GetRegistrationStatus).await?;
                match self.context.registration_events.wait().await {
                    RegistrationStatus::RegisteredHome | RegistrationStatus::RegisteredRoaming => {
                        break
                    }
                    _ => Timer::after(Duration::from_millis(200)).await,
                }
            }
            Ok(())
        };

        let warn_on_long_wait = async {
            for i in 1.. {
                Timer::after(Duration::from_secs(20)).await;
                log::warn!(
                    "modem registration seems to be taking a long time ({}s)...",
                    i * 20
                );
            }
        };

        select_biased! {
            r = wait_for_registration.fuse() => r,
            _ = warn_on_long_wait.fuse() => unreachable!(),
        }
    }

    pub async fn connect_tcp(
        &mut self,
        host: &str,
        port: u16,
    ) -> Result<TcpStream<'c>, ConnectError> {
        let tcp_context = self.context.tcp.claim().unwrap();
        TcpStream::connect(
            tcp_context,
            host,
            port,
            &self.context.drop_channel,
            self.context.commands(),
        )
        .await
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
            Duration::from_secs(20),
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
        self.power_signal.broadcast(PowerState::Sleeping);
        self.power.sleep().await;
    }

    pub async fn wake(&mut self) {
        self.power.wake().await;
        self.power_signal.broadcast(PowerState::On);
    }
}
