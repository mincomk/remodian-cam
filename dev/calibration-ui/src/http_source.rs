use std::sync::mpsc::{self, Receiver};
use std::thread;

#[derive(serde::Deserialize)]
struct CameraDto {
    index: u32,
    name: String,
}

pub struct HttpSource {
    pub base_url: String,
    pub cameras: Vec<(u32, String)>,
    pub camera_index: u32,
    pub status: String,
    cameras_rx: Option<Receiver<Result<Vec<(u32, String)>, String>>>,
    capture_rx: Option<Receiver<Result<image::RgbImage, String>>>,
}

impl Default for HttpSource {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            cameras: Vec::new(),
            camera_index: 0,
            status: String::new(),
            cameras_rx: None,
            capture_rx: None,
        }
    }
}

impl HttpSource {
    pub fn fetch_cameras(&mut self) {
        let url = format!("{}/cameras", self.base_url.trim_end_matches('/'));
        let (tx, rx) = mpsc::channel();
        self.cameras_rx = Some(rx);
        self.status = "Fetching cameras…".into();
        thread::spawn(move || {
            let result = (|| -> Result<Vec<(u32, String)>, String> {
                let resp = ureq::get(&url).call().map_err(|e| e.to_string())?;
                let dtos: Vec<CameraDto> =
                    resp.into_body().read_json().map_err(|e| e.to_string())?;
                Ok(dtos.into_iter().map(|c| (c.index, c.name)).collect())
            })();
            let _ = tx.send(result);
        });
    }

    pub fn fetch_capture(&mut self) {
        let url = format!(
            "{}/capture?index={}",
            self.base_url.trim_end_matches('/'),
            self.camera_index
        );
        let (tx, rx) = mpsc::channel();
        self.capture_rx = Some(rx);
        self.status = "Capturing…".into();
        thread::spawn(move || {
            let result = (|| -> Result<image::RgbImage, String> {
                let resp = ureq::get(&url).call().map_err(|e| e.to_string())?;
                let bytes = resp.into_body().read_to_vec().map_err(|e| e.to_string())?;
                let img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
                Ok(img.to_rgb8())
            })();
            let _ = tx.send(result);
        });
    }

    /// Poll pending channels. Returns `Some(Ok(rgb))` when a capture completes,
    /// `Some(Err(msg))` on capture failure, or `None` when nothing is ready.
    pub fn poll(&mut self) -> Option<Result<image::RgbImage, String>> {
        if let Some(rx) = &self.cameras_rx {
            if let Ok(result) = rx.try_recv() {
                self.cameras_rx = None;
                match result {
                    Ok(list) => {
                        self.cameras = list;
                        if !self.cameras.iter().any(|(i, _)| *i == self.camera_index) {
                            self.camera_index =
                                self.cameras.first().map(|(i, _)| *i).unwrap_or(0);
                        }
                        self.status.clear();
                    }
                    Err(e) => self.status = format!("Error: {}", e),
                }
            }
        }
        if let Some(rx) = &self.capture_rx {
            if let Ok(result) = rx.try_recv() {
                self.capture_rx = None;
                self.status.clear();
                return Some(result);
            }
        }
        None
    }
}
