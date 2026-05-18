use embassy_net::{
    Stack,
    udp::{PacketMetadata, UdpSocket},
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::{Duration, Instant, with_timeout};
use esp_hal::{
    Async,
    rmt::{Channel as RmtChannel, Tx},
};

use crate::rc5::send_rc5x;

/// IR command: (address, command, toggle, signal)
/// signal: 1 = single fire, 2 = start continuous, 3 = stop
pub static IR_CMD: Channel<CriticalSectionRawMutex, (u8, u8, bool, u8), 1> = Channel::new();

const UDP_PORT: u16 = 5555;

/// IR driver loop. Call from the main task with the hardware RMT channel. Never returns.
pub async fn ir_driver(ir: &mut RmtChannel<'_, Async, Tx>) -> ! {
    let mut continuous = false;
    let mut cont_addr: u8 = 0;
    let mut cont_cmd: u8 = 0;
    let mut cont_toggle = false;

    loop {
        if continuous {
            let start = Instant::now();
            let _ = send_rc5x(ir, cont_addr, cont_cmd, cont_toggle).await;
            let elapsed = start.elapsed();
            let gap = Duration::from_millis(117)
                .checked_sub(elapsed)
                .unwrap_or(Duration::from_millis(0));

            match with_timeout(gap, IR_CMD.receive()).await {
                Ok((addr, cmd, tog, sig)) => match sig {
                    1 => {
                        let _ = send_rc5x(ir, addr, cmd, tog).await;
                        continuous = false;
                    }
                    2 => {
                        cont_addr = addr;
                        cont_cmd = cmd;
                        cont_toggle = tog;
                    }
                    3 => continuous = false,
                    _ => {}
                },
                Err(_) => {}
            }
        } else {
            let (addr, cmd, tog, sig) = IR_CMD.receive().await;
            match sig {
                1 => {
                    let _ = send_rc5x(ir, addr, cmd, tog).await;
                }
                2 => {
                    cont_addr = addr;
                    cont_cmd = cmd;
                    cont_toggle = tog;
                    continuous = true;
                }
                _ => {}
            }
        }
    }
}

/// UDP listener for 4-byte IR command packets: `[address, command, toggle, signal]`.
#[embassy_executor::task]
pub async fn udp_task(stack: Stack<'static>) -> ! {
    let mut rx_meta = [PacketMetadata::EMPTY; 8];
    let mut rx_buffer = [0u8; 256];
    let mut tx_meta = [PacketMetadata::EMPTY; 1];
    let mut tx_buffer = [0u8; 16];
    let mut buf = [0u8; 32];

    let mut socket = UdpSocket::new(
        stack,
        &mut rx_meta,
        &mut rx_buffer,
        &mut tx_meta,
        &mut tx_buffer,
    );

    if let Err(e) = socket.bind(UDP_PORT) {
        defmt::error!(
            "Failed to bind UDP socket on :{}: {:?}",
            UDP_PORT,
            defmt::Debug2Format(&e)
        );
        loop {
            embassy_time::Timer::after(Duration::from_secs(60)).await;
        }
    }
    defmt::info!("UDP bound on :{}", UDP_PORT);

    loop {
        match socket.recv_from(&mut buf).await {
            Ok((n, _ep)) => {
                if n != 4 {
                    defmt::error!("UDP: expected 4 bytes, got {}", n);
                    continue;
                }
                let (address, command, toggle, signal) = (buf[0], buf[1], buf[2] != 0, buf[3]);
                defmt::info!(
                    "UDP IR: address={}, command={}, toggle={}, signal={}",
                    address,
                    command,
                    toggle as u8,
                    signal
                );
                if IR_CMD.try_send((address, command, toggle, signal)).is_err() {
                    defmt::error!("IR_CMD channel full, dropping packet");
                }
            }
            Err(e) => {
                defmt::error!("UDP recv error: {:?}", defmt::Debug2Format(&e));
            }
        }
    }
}
