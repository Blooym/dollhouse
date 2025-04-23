use crate::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
};

pub async fn delete_upload_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> StatusCode {
    if !state.storage_provider.file_exists(&id).unwrap() {
        return StatusCode::NOT_FOUND;
    }

    state.storage_provider.delete_file(&id).unwrap();
    StatusCode::OK
}
