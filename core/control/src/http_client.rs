use std::sync::atomic::{AtomicBool, Ordering};

use remodian_client::{Command, RemodianClient, RemodianError};

const ADDRESS: u8 = 0x13;

pub struct HttpRemodianClient {
    client: reqwest::Client,
    base_url: String,
    toggle: AtomicBool,
}

impl HttpRemodianClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            toggle: AtomicBool::new(false),
        }
    }

    async fn post_ir(&self, cmd: u8, toggle: bool, signal: u8) -> Result<(), RemodianError> {
        let url = format!(
            "{}/ir?address={}&command={}&toggle={}&signal={}",
            self.base_url,
            ADDRESS,
            cmd,
            toggle as u8,
            signal,
        );
        self.client
            .post(&url)
            .send()
            .await
            .map_err(RemodianError::Http)?
            .error_for_status()
            .map_err(RemodianError::Http)?;
        Ok(())
    }
}

#[allow(async_fn_in_trait)]
impl RemodianClient for HttpRemodianClient {
    async fn call(&self, cmd: Command) -> Result<(), RemodianError> {
        let t = !self.toggle.fetch_xor(true, Ordering::Relaxed);
        self.post_ir(cmd.code() as u8, t, 1).await
    }

    async fn start(&self, cmd: Command) -> Result<(), RemodianError> {
        let t = !self.toggle.fetch_xor(true, Ordering::Relaxed);
        self.post_ir(cmd.code() as u8, t, 2).await
    }

    async fn stop(&self) -> Result<(), RemodianError> {
        self.post_ir(0, false, 3).await
    }
}
