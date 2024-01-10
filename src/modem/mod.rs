mod command;
mod context;
pub mod power;

use crate::{
    at_command::{
        ate, cbatchk, ccid,
        cedrxs::{self, AcTType, EDRXSetting, EdrxCycleLength},
        cereg,
        cfgri::{self, RiPinMode},
        cgmr, cgnapn,
        cgnsmod::{self, WorkMode},
        cgnspwr, cgnsurc, cgreg, cifsrex, ciicr, cipmux, cipshut,
        cmee::{self, CMEErrorMode},
        cmnb::{self, NbMode},
        cnmp, cops,
        cpsi::{self},
        creg, csclk, csq, cstt, gsn,
        ifc::{self, FlowControl},
        ipr::{self, BaudRate},
        unsolicited::{NetworkRegistration, RegistrationStatus},
        At, AtRequest, BearerSettings, NetworkMode,
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
use embassy_time::{with_timeout, Duration, Timer};
use futures::{select_biased, FutureExt};
use heapless::{String, Vec};

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
    automatic_registration: bool,
    user_network_priority: Vec<RadioAccessTechnology, 3>,
    current_network_priority: Vec<RadioAccessTechnology, 3>,
    // Time given to each RAT before trying the next
    auto_reg_timeout: Duration,
}

const MODEM_POWER_TIMEOUT: Duration = Duration::from_secs(30);
const NET_REG_DEFAULT: NetworkRegistration = NetworkRegistration {
    status: RegistrationStatus::NotRegistered,
    lac: None,
    ci: None,
};

/// Helper macro that repeatedly attempts to evaluate an expression that returns a result.
///
/// Returns the Result yielded by the expression if
/// - the expression returns `Ok` at any point,
/// - or the expression returns `Err` $attempts time in a row.
macro_rules! try_retry {
    (($label:literal, $attempts:literal, $delay: expr), $e:expr) => {{
        let mut attempt = 0;
        loop {
            let r = $e;

            if r.is_ok() || attempt >= $attempts {
                break r;
            }

            attempt += 1;
            log::warn!(
                "{} failed, attempt {}/{}, retrying after {:?}",
                $label,
                attempt,
                $attempts,
                $delay
            );
            Timer::after($delay).await;
        }
    }};
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
            apn: None,
            ap_username: "",
            ap_password: "",
            automatic_registration: false,
            user_network_priority: [
                RadioAccessTechnology::LteCatM1,
                RadioAccessTechnology::Gsm,
                RadioAccessTechnology::LteNbIot,
            ]
            .into_iter()
            .collect(),
            current_network_priority: [
                RadioAccessTechnology::LteCatM1,
                RadioAccessTechnology::Gsm,
                RadioAccessTechnology::LteNbIot,
            ]
            .into_iter()
            .collect(),
            auto_reg_timeout: Duration::from_secs(2 * 60),
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

    pub async fn init(&mut self, config: RegistrationConfig) -> Result<(), Error> {
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

        match config.network_mode {
            NetworkModeConfig::Automatic { priority, timeout } => {
                if let Some(prio) = priority {
                    self.current_network_priority = prio;
                }
                self.automatic_registration = true;
                self.auto_reg_timeout = timeout;
            }
            NetworkModeConfig::Manual {
                network_mode,
                nb_mode,
            } => {
                commands.run(cnmp::SetNetworkMode(network_mode)).await?;
                commands.run(cmnb::SetNbMode(nb_mode)).await?;
            }
        }

        commands.run(cfgri::ConfigureRiPin(RiPinMode::On)).await?;
        commands.run(cbatchk::EnableVBatCheck(true)).await?;

        let configure_edrx = cedrxs::ConfigureEDRX::from(config.edrx);
        let _ = try_retry!(
            ("CEDRX", 5, Duration::from_millis(200)),
            commands.run(configure_edrx).await
        );
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

        let mut commands = self.commands.lock().await;

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
            .run(cmee::ConfigureCMEErrors(CMEErrorMode::Numeric))
            .await?;

        // CREG, CEREG, and CGREG are each necessary based on what network mode we're using
        // (GSM, LTE, etc). But for simplicity's sake we set up URCs for all of them. This is also
        // what Simcom recommends. These commands can fail spuriously though, so we run each in a
        // retry loop up to 5 times.
        try_retry!(
            ("CREG", 5, Duration::from_secs(1)),
            commands
                .run(creg::ConfigureRegistrationUrc::EnableRegLocation)
                .await
        )?;
        try_retry!(
            ("CEREG", 5, Duration::from_secs(1)),
            commands
                .run(cereg::ConfigureRegistrationUrc::EnableReg)
                .await
        )?;
        try_retry!(
            ("CGREG", 5, Duration::from_secs(1)),
            commands
                .run(cgreg::ConfigureRegistrationUrc::EnableRegLocation)
                .await
        )?;

        if self.automatic_registration {
            let active_mode = self.automatic_registration(&commands).await?;

            // re-order the priority list
            if let Some(index) = self
                .current_network_priority
                .iter()
                .position(|mode| *mode == active_mode)
            {
                let element = self.current_network_priority.remove(index);
                self.current_network_priority
                    .insert(0, element)
                    .expect("we just removed an element");
            }
        } else {
            self.wait_for_registration().await?;
        }
        log::info!("registered to network");

        commands.run(cipmux::EnableMultiIpConnection(true)).await?;
        commands.run(cipshut::ShutConnections).await?;

        let apn = match &self.apn {
            Some(apn) => apn,
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

        // datasheet specifies 85 seconds max response time
        commands
            .run_with_timeout(Some(Duration::from_secs(86)), ciicr::StartGprs)
            .await?;

        let (_ip, _) = commands.run(cifsrex::GetLocalIpExt).await?;

        log::info!("modem successfully activated");
        Ok(())
    }

    /// Resets the network priority to the priority provided when initializing [Modem::init] with [NetworkModeConfig::Automatic]
    ///
    /// If not initialized with [NetworkModeConfig::Automatic], this function has no effect
    pub fn reset_network_priority(&mut self) {
        self.current_network_priority = self.user_network_priority.clone();
    }

    /// Connect to the first available radio access technology (RAT).
    /// If connected using LTE-CatM or GSM, set that RAT as first priority for next registration attempt
    async fn automatic_registration(
        &self,
        commands: &CommandRunnerGuard<'_>,
    ) -> Result<RadioAccessTechnology, Error> {
        for mode in &self.current_network_priority {
            match mode {
                RadioAccessTechnology::LteCatM1 => {
                    commands.run(cnmp::SetNetworkMode(NetworkMode::Lte)).await?;
                    commands.run(cmnb::SetNbMode(NbMode::CatM)).await?;
                }
                RadioAccessTechnology::Gsm => {
                    commands.run(cnmp::SetNetworkMode(NetworkMode::Gsm)).await?;
                }
                RadioAccessTechnology::LteNbIot => {
                    commands.run(cnmp::SetNetworkMode(NetworkMode::Lte)).await?;
                    commands.run(cmnb::SetNbMode(NbMode::NbIot)).await?;
                }
            }

            log::info!("Trying {:?}...", mode);
            match with_timeout(self.auto_reg_timeout, self.wait_for_registration()).await {
                Ok(Ok(_)) => {
                    log::info!("Registered using {:?}", mode);
                    return Ok(*mode);
                }
                Ok(Err(_)) => {
                    // this should never happen since wait_for_registration timeout is longer than 2 min
                }
                Err(_) => {}
            }
        }

        return Err(Error::Timeout);
    }

    pub async fn deactivate(&mut self) {
        self.power_signal.broadcast(PowerState::Off);
        self.context.registration_events.signal(NET_REG_DEFAULT);
        self.context.tcp.disconnect_all().await;

        if with_timeout(MODEM_POWER_TIMEOUT, self.power.disable())
            .await
            .is_err()
        {
            log::warn!("timeout while powering off the modem");
        }
    }

    pub async fn reset(&mut self) {
        self.power_signal.broadcast(PowerState::Off);
        self.context.registration_events.signal(NET_REG_DEFAULT);
        self.context.tcp.disconnect_all().await;
        self.power.reset().await;
    }

    /// Wait until the modem has registered to a cell tower.
    pub async fn wait_for_registration(&self) -> Result<(), Error> {
        log::debug!("waiting for cell registration");
        let wait_for_registration = async move {
            self.context
                .registration_events
                .compare_wait(|r| {
                    [
                        RegistrationStatus::RegisteredHome,
                        RegistrationStatus::RegisteredRoaming,
                    ]
                    .contains(&r.status)
                })
                .await;
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
            _ = wait_for_registration.fuse() => Ok(()),
            _ = warn_on_long_wait.fuse() => unreachable!(),
            _ = Timer::after(Duration::from_secs(10 * 60)).fuse() => Err(Error::Timeout),
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

        // Galilean seems to be off by default
        self.commands
            .lock()
            .await
            .run(cgnsmod::SetGnssWorkModeSet {
                glonass: WorkMode::Start,
                beidou: WorkMode::Start,
                galilean: WorkMode::Start,
            })
            .await?;

        Ok(Some(Gnss::new(
            reports,
            self.context.power_signal.subscribe(),
            &self.context.drop_channel,
            Duration::from_secs(20),
        )))
    }

    /// Sync the network time protocol
    pub async fn sync_ntp(&mut self, ntp_server: &str, timezone: u16) -> Result<(), Error> {
        let apn = self.apn.as_ref().ok_or(Error::NoApn)?.clone();

        let commands = self.commands.lock().await;

        commands
            .run(BearerSettings {
                cmd_type: crate::at_command::CmdType::SetBearerParameters,
                con_param_type: crate::at_command::ConParamType::Apn,
                apn: apn.clone(),
            })
            .await?;
        commands
            .run(BearerSettings {
                cmd_type: crate::at_command::CmdType::OpenBearer,
                con_param_type: crate::at_command::ConParamType::Apn,
                apn,
            })
            .await?;
        commands
            .run(crate::at_command::cntpcid::SetGprsBearerProfileId(1))
            .await?;
        commands
            .run(crate::at_command::cntp::SynchronizeNetworkTime {
                ntp_server: ntp_server.into(),
                timezone,
                cid: 1,
            })
            .await?;
        commands.run(crate::at_command::cntp::Execute).await?;

        Ok(())
    }

    /// According to docs, you should first [Modem::sync_ntp]
    pub async fn download_xtra(&mut self, url: &str) -> Result<(), Error> {
        self.commands
            .lock()
            .await
            .run(crate::at_command::cnact::SetAppNetwork {
                mode: crate::at_command::cnact::CnactMode::Active,
                apn: self.apn.as_ref().ok_or(Error::NoApn)?.clone(),
            })
            .await?;

        // sometimes we aren't able to download the file the first couple of times
        try_retry!(
            ("download xtra", 5, Duration::from_millis(200)),
            self.commands
                .lock()
                .await
                .run(crate::at_command::httptofs::DownloadToFileSystem {
                    // unclear which xtra file to use, the size differs depending on server
                    // so they might contain more/different data or different satellite networks
                    // also, sometimes the server is scuffed
                    url: url.into(),
                    file_path: "/customer/xtra3grc.bin".into(),
                })
                .await?
                .1
                .status_code
                .success()
        )
        .map_err(Error::Httptofs)
    }

    /// Enable the use of XTRA file for faster, more accurate GNSS fixes. Similar to assisted gps.
    ///
    /// Before calling this function, make sure the XTRA file has been downloaded. [Modem::download_xtra]
    pub async fn cold_start_with_xtra(&mut self) -> Result<(), Error> {
        self.commands
            .lock()
            .await
            .run(crate::at_command::cgnscpy::CopyXtraFile)
            .await?
            .0
            .success()?;
        self.commands
            .lock()
            .await
            .run(crate::at_command::cgnsxtra::GnssXtra(
                crate::at_command::cgnsxtra::ToggleXtra::Enable,
            ))
            .await?;

        self.commands
            .lock()
            .await
            .run(crate::at_command::cgnscold::GnssColdStart)
            .await?
            .1
            .success()?;

        Ok(())
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

    /// Run a single AT command on the modem with the specified timeout. Use with care.
    pub async fn run_command_with_timeout<C, Response>(
        &self,
        timeout: Option<Duration>,
        command: C,
    ) -> Result<Response, Error>
    where
        C: AtRequest<Response = Response>,
        Response: ExpectResponse,
    {
        self.commands
            .lock()
            .await
            .run_with_timeout(timeout, command)
            .await
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

    /// Query the current cellular network operator.
    ///
    /// This command can take up to 120 seconds to run.
    pub async fn query_operator_info(&mut self) -> Result<cops::OperatorInfo, Error> {
        // max response time is 120 seconds
        self.run_command_with_timeout(Some(Duration::from_secs(121)), cops::GetOperatorInfo)
            .await
            .map(|(response, _)| response)
    }

    pub async fn query_iccid(&mut self) -> Result<ccid::Iccid, Error> {
        self.run_command(ccid::ShowIccid)
            .await
            .map(|(response, _)| response)
    }

    pub async fn query_imei(&mut self) -> Result<String<16>, Error> {
        self.run_command(gsn::GetImei)
            .await
            .map(|(response, _)| response.imei)
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

/// Configure cellular mobile communication and edrx.
pub struct RegistrationConfig {
    pub network_mode: NetworkModeConfig,
    pub edrx: EDRXConfig,
}

#[derive(PartialEq, Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum RadioAccessTechnology {
    LteCatM1,
    LteNbIot,
    Gsm,
}

#[derive(PartialEq)]
pub enum NetworkModeConfig {
    /// Custom automatic, not Simcom automatic.
    ///
    /// Goes through the priority list in order. Connects to first available RAT, which will also be set as first priority for next time.
    Automatic {
        /// If none, priority will be: Lte-CatM > GSM > Lte-NbIoT
        priority: Option<Vec<RadioAccessTechnology, 3>>,
        /// How much time is given for each radio access technology before trying the next
        timeout: Duration,
    },
    /// The modules built-in modes
    Manual {
        network_mode: NetworkMode,
        nb_mode: NbMode,
    },
}

/// Configuration of Extended Discontinuous Reception mode
pub enum EDRXConfig {
    Disabled,
    Enabled {
        auto_report: bool,
        act_type: AcTType,
        cycle_length: EdrxCycleLength,
    },
}

impl Default for RegistrationConfig {
    fn default() -> Self {
        RegistrationConfig {
            network_mode: NetworkModeConfig::Automatic {
                priority: None,
                timeout: Duration::from_secs(2 * 60),
            },
            edrx: EDRXConfig::Disabled,
        }
    }
}

impl From<EDRXConfig> for cedrxs::ConfigureEDRX {
    fn from(value: EDRXConfig) -> Self {
        match value {
            EDRXConfig::Disabled => cedrxs::ConfigureEDRX {
                n: EDRXSetting::Disable,
                // these values don't matter.
                act_type: AcTType::CatM,
                requested_edrx_value: EdrxCycleLength::_5,
            },
            EDRXConfig::Enabled {
                auto_report,
                act_type,
                cycle_length,
            } => cedrxs::ConfigureEDRX {
                n: if auto_report {
                    EDRXSetting::EnableWithAutoReport
                } else {
                    EDRXSetting::Enable
                },
                act_type,
                requested_edrx_value: cycle_length,
            },
        }
    }
}
