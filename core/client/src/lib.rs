use mockall::automock;
use rumqttc::{AsyncClient, EventLoop, MqttOptions, QoS};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use thiserror::Error;
use tokio::time::{Duration, sleep};

const ADDRESS: u32 = 0x13;

#[derive(Debug, Clone, Copy)]
pub enum Command {
    VolUp,
    VolDown,
    CdOn,
    Off,
    Mute,
}

impl Command {
    pub fn code(self) -> u32 {
        match self {
            Command::VolUp => 0x10,
            Command::VolDown => 0x11,
            Command::CdOn => 0x6B,
            Command::Off => 0xC,
            Command::Mute => 0x0D,
        }
    }
}

#[derive(Error, Debug)]
pub enum RemodianError {
    #[error("MQTT client error: {0}")]
    Mqtt(#[from] rumqttc::ClientError),
    #[error("HTTP client error: {0}")]
    Http(reqwest::Error),
}

#[automock]
#[allow(async_fn_in_trait)]
pub trait RemodianClient {
    /// Fire a single RC5 frame (signal = 1).
    async fn call(&self, cmd: Command) -> Result<(), RemodianError>;

    /// Begin continuous RC5 firing at 117 ms intervals (signal = 2).
    async fn start(&self, cmd: Command) -> Result<(), RemodianError>;

    /// Stop continuous firing (signal = 3).
    async fn stop(&self) -> Result<(), RemodianError>;

    async fn vol_up(&self) -> Result<(), RemodianError> {
        self.call(Command::VolUp).await
    }
    async fn vol_down(&self) -> Result<(), RemodianError> {
        self.call(Command::VolDown).await
    }
    async fn cd_on(&self) -> Result<(), RemodianError> {
        self.call(Command::CdOn).await
    }
    async fn off(&self) -> Result<(), RemodianError> {
        self.call(Command::Off).await
    }
    async fn mute(&self) -> Result<(), RemodianError> {
        self.call(Command::Mute).await
    }
}

#[allow(async_fn_in_trait)]
impl<C: RemodianClient> RemodianClient for Arc<C> {
    async fn call(&self, cmd: Command) -> Result<(), RemodianError> {
        self.as_ref().call(cmd).await
    }

    async fn start(&self, cmd: Command) -> Result<(), RemodianError> {
        self.as_ref().start(cmd).await
    }

    async fn stop(&self) -> Result<(), RemodianError> {
        self.as_ref().stop().await
    }
}

pub async fn repeat<C: RemodianClient>(
    client: &C,
    cmd: Command,
    n: u32,
    delay_ms: u64,
) -> Result<(), RemodianError> {
    client.start(cmd).await?;
    sleep(Duration::from_millis(delay_ms * n as u64)).await;
    client.stop().await
}

pub struct MqttRemodianClient {
    client: AsyncClient,
    toggle: AtomicBool,
}

impl MqttRemodianClient {
    pub fn new(host: &str, port: u16, client_id: &str) -> (Self, EventLoop) {
        let opts = MqttOptions::new(client_id, host, port);
        let (client, event_loop) = AsyncClient::new(opts, 10);
        (
            Self {
                client,
                toggle: AtomicBool::new(false),
            },
            event_loop,
        )
    }
}

impl RemodianClient for MqttRemodianClient {
    async fn call(&self, cmd: Command) -> Result<(), RemodianError> {
        let new_t = !self.toggle.fetch_xor(true, Ordering::Relaxed);
        let payload = format!("{},{},{},1", ADDRESS, cmd.code(), new_t as u8);
        self.client
            .publish("ir/send", QoS::AtLeastOnce, false, payload)
            .await?;
        Ok(())
    }

    async fn start(&self, cmd: Command) -> Result<(), RemodianError> {
        let new_t = !self.toggle.fetch_xor(true, Ordering::Relaxed);
        let payload = format!("{},{},{},2", ADDRESS, cmd.code(), new_t as u8);
        self.client
            .publish("ir/send", QoS::AtLeastOnce, false, payload)
            .await?;
        Ok(())
    }

    async fn stop(&self) -> Result<(), RemodianError> {
        self.client
            .publish("ir/send", QoS::AtLeastOnce, false, "0,0,0,3")
            .await?;
        Ok(())
    }
}
