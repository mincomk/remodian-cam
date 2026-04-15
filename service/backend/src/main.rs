use std::convert::Infallible;
use std::sync::{
    Arc,
    atomic::{AtomicU32, AtomicBool, Ordering},
};
use std::time::Duration;

use axum::{
    Json, Router,
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
    routing::{get, post},
};
use parking_lot::Mutex;
use remodian_client::{Command, MqttRemodianClient, RemodianClient};
use remodian_control::{
    AnyClient, HttpRemodianClient, VolumeState, control_loop, fetch_volume_task,
};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tokio_stream::StreamExt as _;
use tokio_stream::wrappers::BroadcastStream;
use tower_http::cors::CorsLayer;

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(clap::Parser)]
struct Cli {
    /// Crop spec passed to compvis-engine, e.g. "10,20,30,40+50,60,70,80".
    /// Also configurable via CROP env var.
    #[clap(long, env = "CROP")]
    crop: String,

    #[clap(long, env = "CAM_URL")]
    cam_url: Option<String>,

    #[clap(long, env = "IR_URL")]
    ir_url: Option<String>,

    /// Port to listen on (default: 3000).
    /// Also configurable via PORT env var.
    #[clap(long, env = "PORT", default_value = "3000")]
    port: u16,
}

// ── Shared state ──────────────────────────────────────────────────────────────

struct AppState {
    volume_state: Arc<Mutex<VolumeState>>,
    desired_volume: Arc<AtomicU32>,
    is_automatic: Arc<AtomicBool>,
    client: Arc<AnyClient>,
    sse_tx: broadcast::Sender<String>,
}

// ── Serialization types ───────────────────────────────────────────────────────

#[derive(Serialize)]
struct VolumeSnapshot {
    current: u32,
    expected: u32,
    desired: u32,
    off: bool,
    automatic: bool,
}

#[derive(Deserialize)]
struct SetVolumeBody {
    volume: u32,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn get_volume(State(state): State<Arc<AppState>>) -> Json<VolumeSnapshot> {
    Json(read_snapshot(&state))
}

async fn sse_handler(
    State(state): State<Arc<AppState>>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let rx = state.sse_tx.subscribe();
    let stream = BroadcastStream::new(rx)
        .filter_map(|msg| msg.ok().map(|data| Ok(Event::default().data(data))));
    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn set_volume(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SetVolumeBody>,
) -> axum::http::StatusCode {
    state.desired_volume.store(body.volume, Ordering::Relaxed);
    state.is_automatic.store(true, Ordering::Relaxed);
    axum::http::StatusCode::NO_CONTENT
}

async fn volume_up(State(state): State<Arc<AppState>>) -> axum::http::StatusCode {
    state.is_automatic.store(false, Ordering::Relaxed);
    if let Err(e) = state.client.vol_up().await {
        eprintln!("volume_up error: {e}");
        return axum::http::StatusCode::INTERNAL_SERVER_ERROR;
    }
    axum::http::StatusCode::NO_CONTENT
}

async fn volume_down(State(state): State<Arc<AppState>>) -> axum::http::StatusCode {
    state.is_automatic.store(false, Ordering::Relaxed);
    if let Err(e) = state.client.vol_down().await {
        eprintln!("volume_down error: {e}");
        return axum::http::StatusCode::INTERNAL_SERVER_ERROR;
    }
    axum::http::StatusCode::NO_CONTENT
}

async fn rapid_up(State(state): State<Arc<AppState>>) -> axum::http::StatusCode {
    state.is_automatic.store(false, Ordering::Relaxed);
    if let Err(e) = state.client.start(Command::VolUp).await {
        eprintln!("rapid_up error: {e}");
        return axum::http::StatusCode::INTERNAL_SERVER_ERROR;
    }
    axum::http::StatusCode::NO_CONTENT
}

async fn rapid_down(State(state): State<Arc<AppState>>) -> axum::http::StatusCode {
    state.is_automatic.store(false, Ordering::Relaxed);
    if let Err(e) = state.client.start(Command::VolDown).await {
        eprintln!("rapid_down error: {e}");
        return axum::http::StatusCode::INTERNAL_SERVER_ERROR;
    }
    axum::http::StatusCode::NO_CONTENT
}

async fn rapid_stop(State(state): State<Arc<AppState>>) -> axum::http::StatusCode {
    if let Err(e) = state.client.stop().await {
        eprintln!("rapid_stop error: {e}");
        return axum::http::StatusCode::INTERNAL_SERVER_ERROR;
    }
    axum::http::StatusCode::NO_CONTENT
}

async fn power_on(State(state): State<Arc<AppState>>) -> axum::http::StatusCode {
    state.is_automatic.store(false, Ordering::Relaxed);
    if let Err(e) = state.client.cd_on().await {
        eprintln!("power_on error: {e}");
        return axum::http::StatusCode::INTERNAL_SERVER_ERROR;
    }
    axum::http::StatusCode::NO_CONTENT
}

async fn power_off(State(state): State<Arc<AppState>>) -> axum::http::StatusCode {
    state.is_automatic.store(false, Ordering::Relaxed);
    if let Err(e) = state.client.off().await {
        eprintln!("power_off error: {e}");
        return axum::http::StatusCode::INTERNAL_SERVER_ERROR;
    }
    axum::http::StatusCode::NO_CONTENT
}

async fn set_manual(State(state): State<Arc<AppState>>) -> axum::http::StatusCode {
    state.is_automatic.store(false, Ordering::Relaxed);
    axum::http::StatusCode::NO_CONTENT
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn read_snapshot(state: &AppState) -> VolumeSnapshot {
    let (current, expected, is_off) = {
        let vs = state.volume_state.lock();
        (vs.volume, vs.expected_volume(), vs.is_off)
    };
    let desired = state.desired_volume.load(Ordering::Relaxed);
    let automatic = state.is_automatic.load(Ordering::Relaxed);
    VolumeSnapshot {
        current,
        expected,
        desired,
        off: is_off,
        automatic,
    }
}

// ── Main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    use clap::Parser as _;

    let cli = Cli::parse();

    let crops: Vec<&str> = cli.crop.split('+').map(|s| s.trim()).collect();
    if crops.len() != 2 {
        eprintln!("Please provide exactly two crops separated by '+'.");
        std::process::exit(1);
    }
    let crop1 = crops[0].to_string();
    let crop2 = crops[1].to_string();

    let volume_state = Arc::new(Mutex::new(VolumeState::new()));
    let desired_volume = Arc::new(AtomicU32::new(60u32));
    let is_automatic = Arc::new(AtomicBool::new(false));

    // SSE broadcast channel (capacity: 64 messages)
    let (sse_tx, _) = broadcast::channel::<String>(64);

    let client: Arc<AnyClient> = if let Some(ref url) = cli.ir_url {
        println!("IR client: HTTP → {url}");
        Arc::new(AnyClient::Http(HttpRemodianClient::new(url)))
    } else {
        println!("IR client: MQTT → 192.168.1.103:1883");
        let (mqtt_client, mut event_loop) =
            MqttRemodianClient::new("192.168.1.103", 1883, "remodian_backend");
        tokio::spawn(async move {
            loop {
                if let Err(e) = event_loop.poll().await {
                    eprintln!("MQTT event loop error: {e}");
                }
            }
        });
        Arc::new(AnyClient::Mqtt(mqtt_client))
    };

    // Spawn background tasks
    tokio::spawn(fetch_volume_task(
        volume_state.clone(),
        crop1,
        crop2,
        cli.cam_url,
        is_automatic.clone(),
    ));

    tokio::spawn(control_loop(
        volume_state.clone(),
        desired_volume.clone(),
        is_automatic.clone(),
        Arc::clone(&client),
    ));

    // SSE broadcaster: publish volume snapshot every 100ms
    {
        let volume_state = volume_state.clone();
        let desired_volume = desired_volume.clone();
        let is_automatic = is_automatic.clone();
        let tx = sse_tx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(100));
            loop {
                interval.tick().await;
                let (current, expected, is_off) = {
                    let vs = volume_state.lock();
                    (vs.volume, vs.expected_volume(), vs.is_off)
                };
                let desired = desired_volume.load(Ordering::Relaxed);
                let automatic = is_automatic.load(Ordering::Relaxed);
                let json = serde_json::json!({
                    "current": current,
                    "expected": expected,
                    "desired": desired,
                    "off": is_off,
                    "automatic": automatic,
                })
                .to_string();
                // Ignore send errors (no active subscribers)
                let _ = tx.send(json);
            }
        });
    }

    let state = Arc::new(AppState {
        volume_state,
        desired_volume,
        is_automatic,
        client,
        sse_tx,
    });

    let app = Router::new()
        .route("/events", get(sse_handler))
        .route("/volume", get(get_volume))
        .route("/volume/set", post(set_volume))
        .route("/volume/up", post(volume_up))
        .route("/volume/down", post(volume_down))
        .route("/volume/rapid-up", post(rapid_up))
        .route("/volume/rapid-down", post(rapid_down))
        .route("/volume/stop", post(rapid_stop))
        .route("/power/on", post(power_on))
        .route("/power/off", post(power_off))
        .route("/mode/manual", post(set_manual))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", cli.port);
    println!("Listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
