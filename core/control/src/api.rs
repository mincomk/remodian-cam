pub async fn get_volume(crop1: &str, crop2: &str) -> Result<Option<u32>, String> {
    let string = reqwest::get(
        "http://192.168.1.235:3001/detect?index=2&crop1=".to_string() + crop1 + "&crop2=" + crop2,
    )
    .await
    .map_err(|e| e.to_string())?
    .text()
    .await
    .map_err(|e| e.to_string())?;

    // volume or `None`; treat 0 as "off" (display blank or black)
    let volume = string.trim().parse::<u32>().ok().filter(|&v| v != 0);
    Ok(volume)
}

/// Fetch a JPEG image from `{cam_url}/capture` and detect the volume digit using compvis-engine.
pub async fn get_volume_from_cam(
    cam_url: &str,
    crop1: &str,
    crop2: &str,
) -> Result<Option<u32>, String> {
    let url = format!("{cam_url}/capture");
    let bytes = reqwest::get(&url)
        .await
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;

    let img = image::load_from_memory(&bytes)
        .map_err(|e| e.to_string())?
        .to_rgb8();

    let crops = format!("{crop1}+{crop2}");
    remodian_vision::detect_number(&img, &crops)
        .map(|opt| opt.filter(|&v| v != 0))
        .map_err(|e| e.to_string())
}
