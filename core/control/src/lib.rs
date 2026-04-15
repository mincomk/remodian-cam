pub mod api;
pub mod controller;
pub mod http_client;
pub mod volume_fetcher;

pub use controller::control_loop;
pub use http_client::HttpRemodianClient;
pub use volume_fetcher::{VolumeState, fetch_volume_task};

use remodian_client::{Command, MqttRemodianClient, RemodianClient, RemodianError};

// ── AnyClient — enum dispatch avoids dyn + async-in-trait unsafety ───────────

pub enum AnyClient {
    Mqtt(MqttRemodianClient),
    Http(HttpRemodianClient),
}

#[allow(async_fn_in_trait)]
impl RemodianClient for AnyClient {
    async fn call(&self, cmd: Command) -> Result<(), RemodianError> {
        match self {
            AnyClient::Mqtt(c) => c.call(cmd).await,
            AnyClient::Http(c) => c.call(cmd).await,
        }
    }

    async fn start(&self, cmd: Command) -> Result<(), RemodianError> {
        match self {
            AnyClient::Mqtt(c) => c.start(cmd).await,
            AnyClient::Http(c) => c.start(cmd).await,
        }
    }

    async fn stop(&self) -> Result<(), RemodianError> {
        match self {
            AnyClient::Mqtt(c) => c.stop().await,
            AnyClient::Http(c) => c.stop().await,
        }
    }
}
