use std::{sync::Arc, time::Duration};

use parking_lot::Mutex;
use tokio::time;

#[derive(Debug, Clone, Copy)]
pub struct VolumeState {
    pub volume: u32,
    pub delta: i32,
    pub is_off: bool,

    last_volume: u32,
    last_update: time::Instant,
}

impl VolumeState {
    pub fn new() -> Self {
        Self {
            volume: 0,
            delta: 0,
            is_off: true,
            last_volume: 0,
            last_update: time::Instant::now(),
        }
    }

    pub fn update(&mut self, new_volume: u32) {
        self.delta = new_volume as i32 - self.last_volume as i32;
        self.volume = new_volume;
        self.last_volume = new_volume;
        self.last_update = time::Instant::now();
    }

    pub fn expected_volume(&self) -> u32 {
        let elapsed = self.last_update.elapsed().as_secs_f32();
        let expected = self.volume as f32 + self.delta as f32 * elapsed;
        expected.round() as u32
    }
}

pub async fn fetch_volume_task(
    val: Arc<Mutex<VolumeState>>,
    crop1: String,
    crop2: String,
    cam_url: Option<String>,
    is_automatic: Arc<std::sync::atomic::AtomicBool>,
) {
    use std::sync::atomic::Ordering;

    let min_interval = Duration::from_millis(200);

    loop {
        let start = time::Instant::now();

        let volume = if let Some(url) = &cam_url {
            crate::api::get_volume_from_cam(url, &crop1, &crop2).await
        } else {
            crate::api::get_volume(&crop1, &crop2).await
        }
        .inspect_err(|e| {
            eprintln!("Error fetching volume: {e}");
            // On error, switch to manual mode and mark as off
            is_automatic.store(false, Ordering::Relaxed);
        })
        .unwrap_or(None);

        match volume {
            Some(v) => {
                let mut state = val.lock();
                state.is_off = false;
                state.update(v);
            }
            None => {
                let mut state = val.lock();
                state.is_off = true;
            }
        }

        let elapsed = start.elapsed();
        if elapsed < min_interval {
            time::sleep(min_interval - elapsed).await;
        }
    }
}
