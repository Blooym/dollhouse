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
use rand::seq::IndexedRandom;
use serde::Serialize;
use std::str::FromStr;
use tracing::error;

const FALLBACK_ENABLED_MIME: Mime = STAR_STAR;

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
    let (infer_str, infer_ext) = match infer::get(&data) {
        Some(f) => (f.mime_type(), f.extension()),
        None => {
            if state
                .upload_allowed_mimetypes
                .contains(&FALLBACK_ENABLED_MIME)
            {
                (APPLICATION_OCTET_STREAM.essence_str(), "")
            } else {
                return Err((
                    StatusCode::UNSUPPORTED_MEDIA_TYPE,
                    "Your file was rejected because the MIME type could not be determined.",
                ));
            }
        }
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
        "{}{}{}",
        Cryptography::hash_bytes(&data, &state.persisted_salt)
            .unwrap()
            .get(..10)
            .unwrap(),
        if !infer_ext.is_empty() { "." } else { "" },
        infer_ext
    );

    let url = state.public_base_urls.choose(&mut rand::rng()).ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "No public base URL configured.",
    ))?;

    match state
        .storage
        .write()
        .await
        .save_upload(&filename, &data)
        .await
    {
        Ok(decryption_key) => Ok(Json(CreateUploadResponse {
            mimetype: infer_str,
            url: format!(
                "{}://{}/upload/{}?key={}",
                url.scheme(),
                url.port()
                    .map_or(url.host_str().unwrap().to_string(), |f| format!(
                        "{}:{}",
                        url.host_str().unwrap(),
                        f,
                    )),
                filename,
                decryption_key
            ),
            id: filename,
            key: decryption_key,
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
