mod command;
mod context;

use embassy_executor::time::{with_timeout, Duration, Timer};
use heapless::Vec;

use crate::{
    at_command::{
        request::*,
        unsolicited::{ConnectionMessage, RegistrationStatus},
    },
    drop::{AsyncDrop, DropMessage},
    gnss::Gnss,
    pump::{DropPump, RxPump, TxPump},
    read::{ModemReader, Read},
    tcp::TcpStream,
    voltage::VoltageWarner,
    write::Write,
    Error, ModemPower,
};
pub use command::{CommandRunner, CommandRunnerGuard, RawAtCommand};
pub use context::*;

pub struct Uninitialized;
pub struct Disabled;
pub struct Enabled;
pub struct Sleeping;

pub struct Modem<'c, P> {
    context: &'c ModemContext,
    commands: CommandRunner<'c>,
    power: P,
}

impl<'c, P: ModemPower> Modem<'c, P> {
    pub async fn new<R: Read, W: Write>(
        rx: R,
        tx: W,
        power: P,
        context: &'c ModemContext,
    ) -> Result<(Modem<'c, P>, TxPump<'c, W>, RxPump<'c, R>, DropPump<'c>), Error> {
        let modem = Modem {
            commands: context.commands(),
            context,
            power,
        };

        let rx_pump = RxPump {
            reader: ModemReader::new(rx),
            generic_response: context.generic_response.sender(),
            registration_events: &context.registration_events,
            tcp: &context.tcp,
            gnss: context.gnss_slot.peek(),
            voltage_warning: context.voltage_slot.peek(),
        };

        let tx_pump = TxPump {
            writer: tx,
            commands: context.commands.receiver(),
        };

        let drop_pump = DropPump { context };

        Ok((modem, tx_pump, rx_pump, drop_pump))
    }

    pub async fn init(&mut self) -> Result<(), Error> {
        self.power.disable().await;
        self.power.enable().await;

        let commands = self.commands.lock().await;
        let set_flow_control = SetFlowControl {
            dce_by_dte: FlowControl::Hardware,
            dte_by_dce: FlowControl::Hardware,
        };

        for _ in 0..5 {
            match with_timeout(Duration::from_millis(2000), async {
                commands.run(set_flow_control).await
            })
            .await
            {
                Ok(Ok(_)) => break,
                _ => {}
            }
        }
        commands.run(csclk::SetSlowClock(false)).await?;
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
                _ => Timer::after(Duration::from_millis(200 as u64)).await,
            }
        }
        commands.run(configure_edrx).await?;

        self.power.disable().await;
        Ok(())
    }

    pub async fn activate(&mut self) -> Result<(), Error> {
        self.power.enable().await;
        let set_flow_control = ifc::SetFlowControl {
            dce_by_dte: FlowControl::Hardware,
            dte_by_dce: FlowControl::Hardware,
        };

        let commands = self.commands.lock().await;

        for _ in 0..5 {
            match with_timeout(Duration::from_millis(2000), async {
                commands.run(set_flow_control).await
            })
            .await
            {
                Ok(Ok(_)) => break,
                _ => {}
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

    async fn wait_for_registration(&self, commands: &CommandRunnerGuard<'_>) -> Result<(), Error> {
        loop {
            match with_timeout(Duration::from_millis(2000), async {
                commands.run(cgreg::GetRegistrationStatus).await
            })
            .await
            {
                Err(_) => continue,
                _ => {}
            }
            match self.context.registration_events.wait().await {
                RegistrationStatus::RegisteredHome | RegistrationStatus::RegisteredRoaming => {
                    break;
                }
                _ => Timer::after(Duration::from_millis(200 as u64)).await,
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

    pub async fn connect_tcp(&mut self, host: &str, port: u16) -> Result<TcpStream<'c>, Error> {
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

        loop {
            match tcp_context.events().recv().await {
                ConnectionMessage::Connected => break,
                ConnectionMessage::ConnectionFailed => panic!("connection failed"), //TODO
                _ => {}
            }
        }

        Ok(TcpStream {
            _drop: AsyncDrop::new(
                &self.context.drop_channel,
                DropMessage::Connection(tcp_context.ordinal()),
            ),
            token: tcp_context,
            commands: self.commands.clone(),
            closed: false,
            buffer: Vec::new(),
        })
    }

    pub async fn claim_gnss(&mut self) -> Result<Option<Gnss<'c>>, Error> {
        let reports = match self.context.gnss_slot.claim() {
            Some(reports) => reports,
            None => return Ok(None),
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

        Ok(Some(Gnss {
            _drop: AsyncDrop::new(&self.context.drop_channel, DropMessage::Gnss),
            reports,
        }))
    }

    pub async fn claim_voltage_warner(&mut self) -> Option<VoltageWarner<'c>> {
        VoltageWarner::take(&self.context.voltage_slot)
    }
}
