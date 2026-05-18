use std::sync::atomic::{AtomicBool, Ordering};

use remodian_client::{Command, RemodianClient, RemodianError};
use tokio::net::UdpSocket;

const ADDRESS: u8 = 0x13;

pub struct UdpRemodianClient {
    socket: UdpSocket,
    toggle: AtomicBool,
}

impl UdpRemodianClient {
    pub async fn new(addr: &str) -> std::io::Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.connect(addr).await?;
        Ok(Self {
            socket,
            toggle: AtomicBool::new(false),
        })
    }

    async fn send_ir(&self, cmd: u8, toggle: bool, signal: u8) -> Result<(), RemodianError> {
        let packet = [ADDRESS, cmd, toggle as u8, signal];
        self.socket.send(&packet).await.map_err(RemodianError::Udp)?;
        Ok(())
    }
}

#[allow(async_fn_in_trait)]
impl RemodianClient for UdpRemodianClient {
    async fn call(&self, cmd: Command) -> Result<(), RemodianError> {
        let t = !self.toggle.fetch_xor(true, Ordering::Relaxed);
        self.send_ir(cmd.code() as u8, t, 1).await
    }

    async fn start(&self, cmd: Command) -> Result<(), RemodianError> {
        let t = !self.toggle.fetch_xor(true, Ordering::Relaxed);
        self.send_ir(cmd.code() as u8, t, 2).await
    }

    async fn stop(&self) -> Result<(), RemodianError> {
        self.send_ir(0, false, 3).await
    }
}
