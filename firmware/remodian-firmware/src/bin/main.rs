#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

extern crate alloc;
extern crate remodian_firmware;

use alloc::string::ToString;
use embassy_executor::Spawner;
use embassy_net::{Config, Runner, StackResources};
use embassy_time::{Duration, Timer};
use esp_hal::{
    Async,
    clock::CpuClock,
    gpio::Level,
    rmt::{Channel, Rmt, Tx, TxChannelConfig, TxChannelCreator as _},
    time::Rate,
    timer::timg::TimerGroup,
};
use esp_radio::wifi::{ClientConfig, ModeConfig, WifiDevice};
use static_cell::StaticCell;

use esp_backtrace as _;
use esp_println as _;

use remodian_firmware::http::{http_task, ir_driver};

// ── Configuration ────────────────────────────────────────────────────────────

const WIFI_SSID: &str = "wlo1";
const WIFI_PASS: &str = "haskell1234";

// ── Statics ──────────────────────────────────────────────────────────────────

static RADIO_CONTROLLER: StaticCell<esp_radio::Controller<'static>> = StaticCell::new();
static STACK_RESOURCES: StaticCell<StackResources<8>> = StaticCell::new();

// ── Panic handler ────────────────────────────────────────────────────────────

// This creates a default app-descriptor required by the esp-idf bootloader.
esp_bootloader_esp_idf::esp_app_desc!();

// ── Embassy tasks ────────────────────────────────────────────────────────────

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) -> ! {
    runner.run().await
}

// ── Main ─────────────────────────────────────────────────────────────────────

#[allow(clippy::large_stack_frames)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 98768);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);

    // ── WiFi init ────────────────────────────────────────────────────────────

    let radio_init = esp_radio::init().expect("Failed to initialize radio");
    let radio_init = RADIO_CONTROLLER.init(radio_init);

    let (mut wifi_controller, interfaces) =
        esp_radio::wifi::new(radio_init, peripherals.WIFI, Default::default())
            .expect("Failed to initialize Wi-Fi");

    // ── Embassy-net stack ────────────────────────────────────────────────────

    let resources = STACK_RESOURCES.init(StackResources::new());
    let seed = 0xDEAD_BEEF_CAFE_1234u64;
    let (stack, runner) = embassy_net::new(
        interfaces.sta,
        Config::dhcpv4(Default::default()),
        resources,
        seed,
    );

    spawner.spawn(net_task(runner)).unwrap();

    // ── RMT / IR transmitter ─────────────────────────────────────────────────

    let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80))
        .expect("Failed to init RMT")
        .into_async();

    let tx_config = TxChannelConfig::default()
        .with_clk_divider(80)
        .with_idle_output(false)
        .with_idle_output_level(Level::Low)
        .with_carrier_modulation(true)
        .with_carrier_high(1111)
        .with_carrier_low(1111)
        .with_carrier_level(Level::High);

    let mut ir_channel: Channel<'_, Async, Tx> = rmt
        .channel0
        .configure_tx(peripherals.GPIO12, tx_config)
        .expect("Failed to configure RMT TX channel");

    // ── Connect to WiFi ───────────────────────────────────────────────────────

    wifi_controller
        .set_config(&ModeConfig::Client(
            ClientConfig::default()
                .with_ssid(WIFI_SSID.to_string())
                .with_password(WIFI_PASS.to_string()),
        ))
        .expect("Failed to set WiFi config");

    wifi_controller
        .start_async()
        .await
        .expect("Failed to start WiFi");

    wifi_controller
        .connect_async()
        .await
        .expect("Failed to connect WiFi");

    // Wait for DHCP
    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }
    loop {
        if stack.config_v4().is_some() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    // ── Start HTTP server + run IR driver ─────────────────────────────────────

    spawner.must_spawn(http_task(stack, spawner));

    ir_driver(&mut ir_channel).await
}
