use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::IntoResponse,
};
use serde::Deserialize;

/// The response for if a file does not exist or for a decryption failure.
///
/// # Notes:
/// The same response must be given for both scenarios to ensure the file is
/// not confirmed to exist unless the end user actually has the decryption key.
const DECRYPT_OR_NOT_FOUND_RESPONSE: (StatusCode, &str) = (
    StatusCode::NOT_FOUND,
    "This file could not be displayed. Either it does not exist, or your decryption key is invalid.",
);

#[derive(Deserialize)]
pub struct GetUploadQuery {
    /// Decryption key for the upload.
    key: String,
}

pub async fn get_upload_handler(
    query: Query<GetUploadQuery>,
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let storage = state.storage.read().await;

    // Don't bother trying to decrypt if we know the file doesn't exist.
    if !storage.upload_exists(&id).await.unwrap() {
        return DECRYPT_OR_NOT_FOUND_RESPONSE.into_response();
    }

    match storage.get_upload(&id, &query.key).await {
        Ok(bytes) => (
            [
                (
                    header::CONTENT_TYPE,
                    mime_guess::from_path(&id)
                        .first_or_octet_stream()
                        .essence_str(),
                ),
                (header::CACHE_CONTROL, "private, max-age=1800, immutable"),
            ],
            (bytes),
        )
            .into_response(),
        Err(_) => DECRYPT_OR_NOT_FOUND_RESPONSE.into_response(),
    }
}
