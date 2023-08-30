use core::{cmp::min, mem};
use core::future::Future;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::{Receiver, Sender},
    mutex::{Mutex, MutexGuard},
};
use embassy_time::{with_timeout, Duration, TimeoutError};
use heapless::{String, Vec};

use crate::at_command::{AtRequest, AtResponse, ResponseCode};
use crate::log;
use crate::modem::ModemContext;
use crate::Error;

/// The default timeout of AT commands
pub const AT_DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

pub enum RawAtCommand {
    Text(String<256>),
    Binary(Vec<u8, 256>),
}

impl From<String<256>> for RawAtCommand {
    fn from(s: String<256>) -> Self {
        RawAtCommand::Text(s)
    }
}

impl From<&'_ str> for RawAtCommand {
    fn from(s: &'_ str) -> Self {
        RawAtCommand::Text(s.into())
    }
}

impl RawAtCommand {
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            RawAtCommand::Text(s) => s.as_bytes(),
            RawAtCommand::Binary(b) => b,
        }
    }
}

#[derive(Clone)]
pub struct CommandRunner<'a> {
    command_lock: &'a Mutex<CriticalSectionRawMutex, ()>,
    commands: Sender<'a, CriticalSectionRawMutex, RawAtCommand, 4>,
    responses: Receiver<'a, CriticalSectionRawMutex, ResponseCode, 1>,
}

impl<'a> CommandRunner<'a> {
    pub fn create(ctx: &'a ModemContext) -> Self {
        CommandRunner {
            command_lock: &ctx.command_lock,
            commands: ctx.commands.sender(),
            responses: ctx.generic_response.receiver(),
        }
    }
}

pub struct CommandRunnerGuard<'a> {
    _commands_guard: MutexGuard<'a, CriticalSectionRawMutex, ()>,
    runner: &'a CommandRunner<'a>,
    timeout: Option<Duration>,
}

impl<'a> CommandRunner<'a> {
    pub async fn lock(&'a self) -> CommandRunnerGuard<'a> {
        CommandRunnerGuard {
            _commands_guard: self.command_lock.lock().await,
            runner: self,
            timeout: Some(AT_DEFAULT_TIMEOUT),
        }
    }
}

impl<'a> CommandRunnerGuard<'a> {
    /// Run a future with the timeout configured for self
    async fn timeout<T, F: Future<Output = T>>(&self, future: F) -> Result<T, TimeoutError> {
        Ok(match self.timeout {
            Some(timeout) => with_timeout(timeout, future).await?,
            None => future.await,
        })
    }

    /// Send a request to the modem, but do not wait for a response.
    pub async fn send_request<R: AtRequest>(&self, request: &R) -> Result<(), TimeoutError> {
        self.timeout(async {
            self.runner.commands.send(request.encode().into()).await;
        })
        .await
    }

    /// Wait for the modem to return a specific response.
    pub async fn expect_response<T: AtResponse>(&self) -> Result<T, Error> {
        self.timeout(async {
            loop {
                let response = self.runner.responses.recv().await;

                match T::from_generic(response) {
                    Ok(response) => return Ok(response),
                    Err(ResponseCode::Error(error)) => return Err(Error::Sim(error)),
                    Err(unknown_response) => {
                        // TODO: we might want to make this a hard error, if/when we feel confident in
                        // how both the driver and the modem behaves
                        log::warn!("Got unexpected ATResponse: {:?}", unknown_response)
                    }
                }
            }
        })
        .await?
    }

    /// Send raw bytes to the modem, use with care.
    pub async fn send_bytes(&self, mut bytes: &[u8]) {
        while !bytes.is_empty() {
            let mut chunk = Vec::new();
            let n = min(chunk.capacity(), bytes.len());
            chunk.extend_from_slice(&bytes[..n]).unwrap();
            bytes = &bytes[n..];
            self.runner.commands.send(RawAtCommand::Binary(chunk)).await;
        }
    }

    /// Send a request to the modem, and wait for the modem to respond.
    pub async fn run<Request, Response>(&self, command: Request) -> Result<Response, Error>
    where
        Request: AtRequest<Response = Response>,
        Response: ExpectResponse,
    {
        log::trace!("Running AT command: {:?}", command);
        self.send_request(&command).await?;
        log::trace!("Waiting for response for AT command: {:?}", command);
        let result = Response::expect(self).await;
        log::trace!("Completed AT command: {:?}", command);

        if let Err(e) = &result {
            log::error!("AT command {:?} error: {:?}", command, e);
        }

        result
    }

    /// Send a request to the modem and wait for the modem to respond.
    ///
    /// Use the provided timeout value instead of the configured one.
    pub async fn run_with_timeout<Request, Response>(&mut self, mut timeout: Option<Duration>, command: Request) -> Result<Response, Error>

    where
        Request: AtRequest<Response = Response>,
        Response: ExpectResponse,
    {
        mem::swap(&mut self.timeout, &mut timeout);
        let result = self.run(command).await;
        mem::swap(&mut self.timeout, &mut timeout);
        result
    }

    /// Set the timeout of subsequent commands
    ///
    /// Note that the timeout defaults to [AT_DEFAULT_TIMEOUT].
    pub fn with_timeout(self, timeout: Option<Duration>) -> Self {
        Self { timeout, ..self }
    }
}

/// Implemented for (tuples of) AtResponse.
///
/// In order to support AtRequest::Response being a tuple of arbitrary size, we
/// implement the ExpectResponse trait for tuples with as many member as we need.
pub trait ExpectResponse: Sized {
    type Fut<'a>: Future<Output = Result<Self, Error>> + 'a
    where
        Self: 'a;

    fn expect<'a>(runner: &'a CommandRunnerGuard<'a>) -> Self::Fut<'a>;
}

impl<T: AtResponse> ExpectResponse for T {
    type Fut<'a> = impl Future<Output = Result<Self, Error>> + 'a
    where
        Self: 'a;

    fn expect<'a>(runner: &'a CommandRunnerGuard<'a>) -> Self::Fut<'a> {
        runner.expect_response()
    }
}

impl<T: AtResponse, Y: AtResponse> ExpectResponse for (T, Y) {
    type Fut<'a> = impl Future<Output = Result<Self, Error>> + 'a
    where
        Self: 'a;

    fn expect<'a>(runner: &'a CommandRunnerGuard<'a>) -> Self::Fut<'a> {
        async {
            let r1 = runner.expect_response().await?;
            let r2 = runner.expect_response().await?;
            Ok((r1, r2))
        }
    }
}
