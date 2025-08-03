use crate::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
};
use tracing::error;

pub async fn delete_upload_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> StatusCode {
    match state.storage.read().await.upload_exists(&id).await {
        Ok(exists) => {
            if !exists {
                return StatusCode::NOT_FOUND;
            }
        }
        Err(err) => {
            error!("Failed to check if upload exists: {}", err);
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    }

    if let Err(err) = state.storage.write().await.delete_upload(&id).await {
        error!("Failed to delete upload {}: {}", id, err);
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    StatusCode::OK
}
