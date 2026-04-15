use std::{
    sync::{
        Arc,
        atomic::AtomicU32,
    },
    time::Duration,
};

use parking_lot::Mutex;
use remodian_client::{Command, RemodianClient};

use crate::volume_fetcher::VolumeState;

// Position error below this → no command, hold position
const DEADBAND: f32 = 0.5;
const NEAR_TARGET_THRESHOLD: f32 = 8.0;
const NEAR_SLEEP_MS: u64 = 200;

const VERY_NEAR_TARGET_THRESHOLD: f32 = 4.0;
const VERY_NEAR_SLEEP_MS: u64 = 700;

pub async fn control_loop(
    volume_state: Arc<Mutex<VolumeState>>,
    desired_volume: Arc<AtomicU32>,
    is_automatic: Arc<std::sync::atomic::AtomicBool>,
    rd_client: impl RemodianClient,
) {
    use std::sync::atomic::Ordering;

    let mut rapid_firing = false;

    loop {
        // If in manual mode, pause control loop
        if !is_automatic.load(Ordering::Relaxed) {
            if rapid_firing {
                if let Err(e) = rd_client.stop().await {
                    eprintln!("Failed to stop rapid firing: {}", e);
                }
                rapid_firing = false;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        }

        let (current_volume, is_off) = {
            let state = volume_state.lock();
            (state.expected_volume(), state.is_off)
        };

        if is_off {
            if rapid_firing {
                if let Err(e) = rd_client.stop().await {
                    eprintln!("Failed to stop rapid firing: {}", e);
                }
                rapid_firing = false;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        }

        let target_volume = desired_volume.load(Ordering::Relaxed) as f32;

        let error = target_volume as f32 - current_volume as f32;

        // Only send commands if we're outside the deadband
        if error.abs() > DEADBAND {
            let command = if error > 0.0 {
                Command::VolUp
            } else {
                Command::VolDown
            };

            // If we're near the target
            if error.abs() < NEAR_TARGET_THRESHOLD {
                // stop rapid fire and fire one by one
                if rapid_firing {
                    if let Err(e) = rd_client.stop().await {
                        eprintln!("Failed to stop rapid firing: {}", e);
                    }
                    rapid_firing = false;
                } else {
                    // fire one command and sleep for a short duration
                    if let Err(e) = rd_client.call(command).await {
                        eprintln!("Failed to send command: {}", e);
                    }

                    if error.abs() < VERY_NEAR_TARGET_THRESHOLD {
                        tokio::time::sleep(Duration::from_millis(VERY_NEAR_SLEEP_MS)).await;
                    } else {
                        tokio::time::sleep(Duration::from_millis(NEAR_SLEEP_MS)).await;
                    }
                }
            } else {
                if !rapid_firing {
                    // Start rapid firing
                    if let Err(e) = rd_client.start(command).await {
                        eprintln!("Failed to start rapid firing: {}", e);
                    }
                    rapid_firing = true;
                }
            }
        }

        // Sleep for a short duration to prevent spamming commands
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
