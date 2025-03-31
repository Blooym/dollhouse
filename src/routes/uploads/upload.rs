use crate::{AppState, cryptography::Cryptography, mime};
use axum::{
    Json,
    extract::{Multipart, State},
    http::StatusCode,
};
use mime_guess::{
    Mime,
    mime::{APPLICATION_OCTET_STREAM, STAR_STAR},
};
use serde::Serialize;
use std::str::FromStr;
use tracing::error;

const FALLBACK_ENABLED_MIME: Mime = STAR_STAR;
const FALLBACK_MIME_TYPE: Mime = APPLICATION_OCTET_STREAM;
const FALLBACK_FILE_EXTENSION: &str = "unknown";

#[derive(Serialize)]
pub struct CreateUploadResponse {
    url: String,
    id: String,
    key: String,
    mimetype: &'static str,
}

pub async fn create_upload_handler(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<CreateUploadResponse>, (StatusCode, &'static str)> {
    // Get data from first multipart upload.
    let field = match multipart.next_field().await {
        Ok(field) => {
            let Some(field) = field else {
                return Err((StatusCode::BAD_REQUEST, "Multipart field name was not set"));
            };
            field
        }
        Err(_) => return Err((StatusCode::BAD_REQUEST, "Multipart field error")),
    };
    let Ok(data) = field.bytes().await else {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            "Upload is too big to be processed by this server.",
        ));
    };

    // Infer mimetype by magic numbers and reject
    let Ok((infer_str, infer_ext)) = infer::get(&data).map_or_else(
        || {
            // If wildcard mime is enabled, we can fallback to octet stream.
            if state
                .upload_allowed_mimetypes
                .contains(&FALLBACK_ENABLED_MIME)
            {
                Ok((FALLBACK_MIME_TYPE.essence_str(), FALLBACK_FILE_EXTENSION))
            } else {
                Err(())
            }
        },
        |f| Ok((f.mime_type(), f.extension())),
    ) else {
        return Err((
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Your file was rejected because the MIME type could not be determined.",
        ));
    };

    if !mime::is_mime_allowed(
        &Mime::from_str(infer_str).unwrap(),
        &state.upload_allowed_mimetypes,
    ) {
        return Err((
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Your file was rejected because uploading file of this type is not permitted.",
        ));
    }

    // Store file by hash to prevent duplicating uploads.
    let filename = format!(
        "{}.{}",
        Cryptography::hash_from_bytes(&data, &state.persisted_salt)
            .unwrap()
            .get(..10)
            .unwrap(),
        infer_ext
    );

    match state.storage_provider.save_file(&filename, &data) {
        Ok(decryption_key) => Ok(Json(CreateUploadResponse {
            mimetype: infer_ext,
            url: format!(
                "{}://{}/upload/{}?key={}",
                state.public_base_url.scheme(),
                state.public_base_url.port().map_or(
                    state.public_base_url.host_str().unwrap().to_string(),
                    |f| format!("{}:{}", state.public_base_url.host_str().unwrap(), f,)
                ),
                filename,
                decryption_key
            ),
            key: decryption_key,
            id: filename,
        })),
        Err(err) => {
            error!("Error while encrypting or writing file {filename}: {err:?}");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Your file could not be encrypted/written to storage successfully.",
            ))
        }
    }
}
