use axum::{
    Json, Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
    routing::get,
};
use std::sync::Arc;
use remodian_vision::detect_number;
use nokhwa::{
    Camera,
    pixel_format::RgbFormat,
    query,
    utils::{ApiBackend, CameraIndex, RequestedFormat, RequestedFormatType},
};
use serde::{Deserialize, Serialize};
use std::io::Cursor;

#[derive(Serialize)]
struct CameraInfo {
    index: u32,
    name: String,
}

async fn list_cameras() -> Result<Json<Vec<CameraInfo>>, (StatusCode, String)> {
    let cameras = query(ApiBackend::Auto).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to query cameras: {e}"),
        )
    })?;

    let infos = cameras
        .into_iter()
        .map(|c| CameraInfo {
            index: match c.index() {
                CameraIndex::Index(i) => *i,
                CameraIndex::String(_) => u32::MAX,
            },
            name: c.human_name().to_string(),
        })
        .collect();

    Ok(Json(infos))
}

#[derive(Deserialize)]
struct CaptureParams {
    #[serde(default)]
    index: u32,
}

async fn capture_frame(Query(params): Query<CaptureParams>) -> impl IntoResponse {
    let result = tokio::task::spawn_blocking(move || {
        let format =
            RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
        let mut camera = Camera::new(CameraIndex::Index(params.index), format)?;
        camera.open_stream()?;
        let frame = camera.frame()?;
        let decoded = frame.decode_image::<RgbFormat>()?;

        let img = image::RgbImage::from_raw(decoded.width(), decoded.height(), decoded.into_vec())
            .ok_or_else(|| {
                nokhwa::NokhwaError::ReadFrameError("Image buffer size mismatch".into())
            })?;

        let mut jpeg_bytes = Vec::new();
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut Cursor::new(&mut jpeg_bytes), image::ImageFormat::Jpeg)
            .map_err(|e: image::ImageError| nokhwa::NokhwaError::ReadFrameError(e.to_string()))?;

        Ok::<Vec<u8>, nokhwa::NokhwaError>(jpeg_bytes)
    })
    .await;

    match result {
        Ok(Ok(bytes)) => {
            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, "image/jpeg".parse().unwrap());
            (StatusCode::OK, headers, bytes).into_response()
        }
        Ok(Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Camera error: {e}"),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Task error: {e}"),
        )
            .into_response(),
    }
}

#[derive(Clone)]
struct AppState {
    default_crop1: Option<String>,
    default_crop2: Option<String>,
}

#[derive(Deserialize)]
struct DetectParams {
    #[serde(default)]
    index: u32,
    crop1: Option<String>,
    crop2: Option<String>,
}

async fn detect_frame(
    State(state): State<Arc<AppState>>,
    Query(params): Query<DetectParams>,
) -> impl IntoResponse {
    let crop1 = match params.crop1.or_else(|| state.default_crop1.clone()) {
        Some(c) => c,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                "Missing `crop1` query param and DEFAULT_CROP1 env var is not set".to_string(),
            )
                .into_response();
        }
    };
    let crop2 = match params.crop2.or_else(|| state.default_crop2.clone()) {
        Some(c) => c,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                "Missing `crop2` query param and DEFAULT_CROP2 env var is not set".to_string(),
            )
                .into_response();
        }
    };
    let crops = format!("{crop1}+{crop2}");
    let result = tokio::task::spawn_blocking(move || {
        let format =
            RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
        let mut camera = Camera::new(CameraIndex::Index(params.index), format)?;
        camera.open_stream()?;
        let frame = camera.frame()?;
        let decoded = frame.decode_image::<RgbFormat>()?;
        let img = image::RgbImage::from_raw(decoded.width(), decoded.height(), decoded.into_vec())
            .ok_or_else(|| {
                nokhwa::NokhwaError::ReadFrameError("Image buffer size mismatch".into())
            })?;
        Ok::<image::RgbImage, nokhwa::NokhwaError>(img)
    })
    .await;

    let img = match result {
        Ok(Ok(img)) => img,
        Ok(Err(e)) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Camera error: {e}"),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Task error: {e}"),
            )
                .into_response();
        }
    };

    match detect_number(&img, &crops) {
        Ok(Some(n)) => (StatusCode::OK, n.to_string()).into_response(),
        Ok(None) => (StatusCode::OK, "None".to_string()).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, format!("Detection error: {e}")).into_response(),
    }
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        default_crop1: std::env::var("DEFAULT_CROP1").ok(),
        default_crop2: std::env::var("DEFAULT_CROP2").ok(),
    });

    let app = Router::new()
        .route("/cameras", get(list_cameras))
        .route("/capture", get(capture_frame))
        .route("/detect", get(detect_frame))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
    println!("Listening on http://0.0.0.0:3001");
    axum::serve(listener, app).await.unwrap();
}
