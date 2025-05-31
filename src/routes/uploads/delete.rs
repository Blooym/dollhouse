use crate::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
};

pub async fn delete_upload_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> StatusCode {
    if !state.storage.read().await.upload_exists(&id).await.unwrap() {
        return StatusCode::NOT_FOUND;
    }
    state
        .storage
        .write()
        .await
        .delete_upload(&id)
        .await
        .unwrap();
    StatusCode::OK
}
