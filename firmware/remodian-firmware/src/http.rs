use embassy_executor::Spawner;
use embassy_net::{Stack, tcp::TcpSocket};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::Channel,
    semaphore::{GreedySemaphore, Semaphore, SemaphoreReleaser},
};
use embassy_time::{Duration, Instant, with_timeout};
use esp_hal::{
    Async,
    rmt::{Channel as RmtChannel, Tx},
};
use picoserve::{
    AppBuilder, Timeouts,
    extract::Query,
    response::StatusCode,
    routing::{PathRouter, get, post},
};
use serde::Deserialize;

use crate::rc5::send_rc5x;

/// IR command: (address, command, toggle, signal)
/// signal: 1 = single fire, 2 = start continuous, 3 = stop
pub static IR_CMD: Channel<CriticalSectionRawMutex, (u8, u8, bool, u8), 1> = Channel::new();

static CONNECTION_SEMAPHORE: GreedySemaphore<CriticalSectionRawMutex> = GreedySemaphore::new(4);

#[derive(Deserialize)]
struct IrParams {
    address: u8,
    command: u8,
    toggle: u8,
    signal: u8,
}

struct AppProps;

impl AppBuilder for AppProps {
    type PathRouter = impl PathRouter;

    fn build_app(self) -> picoserve::Router<Self::PathRouter> {
        picoserve::Router::new()
            .route("/", get(async || StatusCode::OK))
            .route(
                "/ir",
                post(async |Query(p): Query<IrParams>| {
                    defmt::info!(
                        "Received IR command: address={}, command={}, toggle={}, signal={}",
                        p.address,
                        p.command,
                        p.toggle,
                        p.signal
                    );
                    if let Err(_e) =
                        IR_CMD.try_send((p.address, p.command, p.toggle != 0, p.signal))
                    {
                        defmt::error!("Failed to send IR command.");
                    }
                    StatusCode::OK
                }),
            )
    }
}

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

#[embassy_executor::task]
pub async fn http_task(stack: Stack<'static>, spawner: Spawner) -> ! {
    loop {
        let permit = CONNECTION_SEMAPHORE.acquire(1).await.unwrap();
        defmt::info!("Waiting for incoming connection...");
        if spawner.spawn(accept_connection(stack, permit)).is_err() {
            defmt::error!("Failed to spawn connection task");
        }
    }
}

#[embassy_executor::task(pool_size = 8)]
async fn accept_connection(
    stack: Stack<'static>,
    _permit: SemaphoreReleaser<'static, GreedySemaphore<CriticalSectionRawMutex>>,
) {
    let mut rx_buffer = [0; 1024];
    let mut tx_buffer = [0; 1024];

    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);

    if socket.accept(80).await.is_err() {
        defmt::error!("Failed to accept connection");
        return;
    }

    let mut http_buffer = [0; 2048];

    static CONFIG: picoserve::Config<Duration> = picoserve::Config::new(Timeouts {
        start_read_request: Some(Duration::from_secs(5)),
        persistent_start_read_request: Some(Duration::from_secs(5)),
        read_request: Some(Duration::from_secs(10)),
        write: Some(Duration::from_secs(5)),
    });

    let app = AppProps;
    let app = app.build_app();
    let server = picoserve::Server::new(&app, &CONFIG, &mut http_buffer);

    if let Err(e) = server.serve(socket).await {
        defmt::error!("Error while serving HTTP connection: {:?}", e);
    }
}
