pub fn list_cameras() -> Vec<(u32, String)> {
    use nokhwa::query;
    use nokhwa::utils::{ApiBackend, CameraIndex};

    query(ApiBackend::Auto)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|c| match c.index() {
            CameraIndex::Index(i) => Some((*i, c.human_name().to_string())),
            _ => None,
        })
        .collect()
}

pub fn capture_camera_frame(index: u32) -> Result<image::RgbImage, String> {
    use nokhwa::Camera;
    use nokhwa::pixel_format::RgbFormat;
    use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType};

    let format = RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
    let mut cam = Camera::new(CameraIndex::Index(index), format).map_err(|e| e.to_string())?;
    cam.open_stream().map_err(|e| e.to_string())?;
    let frame = cam.frame().map_err(|e| e.to_string())?;
    frame.decode_image::<RgbFormat>().map_err(|e| e.to_string())
}
