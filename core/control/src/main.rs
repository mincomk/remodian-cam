use std::sync::{
    Arc,
    atomic::{AtomicU32, AtomicBool, Ordering},
};

use eframe::egui;
use parking_lot::Mutex;
use remodian_client::MqttRemodianClient;
use remodian_control::{AnyClient, HttpRemodianClient, VolumeState, control_loop, fetch_volume_task};

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(clap::Parser)]
struct Cli {
    #[clap(long)]
    crop: String,

    /// HTTP URL of the ESP32-CAM (e.g. http://192.168.1.50).
    /// When set, volume is read by fetching `{cam_url}/capture` and running
    /// compvis-engine locally instead of calling the vis-server.
    #[clap(long)]
    cam_url: Option<String>,

    /// HTTP base URL of the IR device (e.g. http://192.168.1.60).
    /// When set, IR commands are sent via HTTP POST /ir instead of MQTT.
    #[clap(long)]
    ir_url: Option<String>,
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() -> eframe::Result {
    use clap::Parser as _;

    let cli = Cli::parse();

    let rt = tokio::runtime::Runtime::new().unwrap();

    let volume_state = Arc::new(Mutex::new(VolumeState::new()));
    let desired_volume = Arc::new(AtomicU32::new(60u32));
    let is_automatic = Arc::new(AtomicBool::new(false));

    let crops: Vec<&str> = cli.crop.split('+').map(|s| s.trim()).collect();
    if crops.len() != 2 {
        eprintln!("Please provide exactly two crops separated by '+'.");
        std::process::exit(1);
    }
    let crop1 = crops[0].to_string();
    let crop2 = crops[1].to_string();

    rt.spawn(fetch_volume_task(
        volume_state.clone(),
        crop1,
        crop2,
        cli.cam_url,
        is_automatic.clone(),
    ));

    let remodian_client = if let Some(url) = cli.ir_url {
        println!("IR client: HTTP → {url}");
        AnyClient::Http(HttpRemodianClient::new(&url))
    } else {
        println!("IR client: MQTT → 192.168.1.103:1883");
        let (mqtt_client, mut event_loop) =
            MqttRemodianClient::new("192.168.1.103", 1883, "remodian_client");
        rt.spawn(async move {
            loop {
                if let Err(e) = event_loop.poll().await {
                    eprintln!("MQTT event loop error: {e}");
                }
            }
        });
        AnyClient::Mqtt(mqtt_client)
    };

    rt.spawn(control_loop(
        volume_state.clone(),
        desired_volume.clone(),
        is_automatic.clone(),
        remodian_client,
    ));

    eframe::run_native(
        "Remodian Control",
        eframe::NativeOptions::default(),
        Box::new(|_cc| {
            Ok(Box::new(App {
                volume_state,
                desired_volume,
                is_automatic,
                pending_desired: 60.0,
            }))
        }),
    )
}

// ── egui App ──────────────────────────────────────────────────────────────────

struct App {
    volume_state: Arc<Mutex<VolumeState>>,
    desired_volume: Arc<AtomicU32>,
    is_automatic: Arc<AtomicBool>,
    pending_desired: f32,
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let current = {
            let state = self.volume_state.lock();
            state.expected_volume()
        };
        let applied = self.desired_volume.load(Ordering::Relaxed);
        let is_automatic = self.is_automatic.load(Ordering::Relaxed);

        ui.heading("Remodian Volume Control");
        ui.separator();

        ui.label(format!("Current Volume: {current}"));
        ui.label(format!("Applied Target: {applied}"));

        // Mode badge
        let mode_text = if is_automatic { "Automatic" } else { "Manual" };
        let mode_color = if is_automatic {
            egui::Color32::LIGHT_BLUE
        } else {
            egui::Color32::LIGHT_GRAY
        };
        ui.colored_label(mode_color, format!("Mode: {mode_text}"));

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Desired Volume:");
            ui.add(egui::Slider::new(&mut self.pending_desired, 0.0..=100.0));
        });

        ui.horizontal(|ui| {
            if ui.button("Apply (Auto)").clicked() {
                self.desired_volume
                    .store(self.pending_desired.round() as u32, Ordering::Relaxed);
                self.is_automatic.store(true, Ordering::Relaxed);
            }

            if ui.button("Manual").clicked() {
                self.is_automatic.store(false, Ordering::Relaxed);
            }
        });

        ui.ctx().request_repaint();
    }
}
