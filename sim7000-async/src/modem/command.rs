use core::cmp::min;
use core::future::Future;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::{Receiver, Sender},
    mutex::{Mutex, MutexGuard},
};
use heapless::{String, Vec};

use crate::at_command::{AtRequest, AtResponse, ResponseCode};
use crate::log;
use crate::modem::ModemContext;
use crate::Error;

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
}

impl<'a> CommandRunner<'a> {
    pub async fn lock(&'a self) -> CommandRunnerGuard<'a> {
        CommandRunnerGuard {
            _commands_guard: self.command_lock.lock().await,
            runner: self,
        }
    }
}

impl<'a> CommandRunnerGuard<'a> {
    pub async fn send_request<C: AtRequest>(&self, request: C) {
        self.runner.commands.send(request.encode().into()).await;
    }

    pub async fn expect_response<T: AtResponse>(&self) -> Result<T, Error> {
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
    }

    pub async fn send_bytes(&self, mut bytes: &[u8]) {
        while !bytes.is_empty() {
            let mut chunk = Vec::new();
            let n = min(chunk.capacity(), bytes.len());
            chunk.extend_from_slice(&bytes[..n]).unwrap();
            bytes = &bytes[n..];
            self.runner.commands.send(RawAtCommand::Binary(chunk)).await;
        }
    }

    pub async fn run<C, Response>(&self, command: C) -> Result<Response, Error>
    where
        C: AtRequest<Response = Response>,
        Response: ExpectResponse,
    {
        self.send_request(command).await;
        Response::expect(self).await
    }
}

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
