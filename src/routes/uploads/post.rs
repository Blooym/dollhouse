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
use tracing::{debug, error};

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
    // Extract upload data from multipart field
    let upload_bytes = {
        let upload_field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => {
                debug!("Rejecting upload - does not contain a valid multipart field");
                return Err((StatusCode::BAD_REQUEST, "Multipart field not found"));
            }
            Err(_) => {
                debug!("Rejecting upload - contains one or more unparseable multipart fields");
                return Err((
                    StatusCode::BAD_REQUEST,
                    "Multipart field could not be parsed",
                ));
            }
        };
        match upload_field.bytes().await {
            Ok(bytes) => bytes,
            Err(_) => {
                debug!(
                    "Rejecting upload - content is larger than the server's maximum allowed size"
                );
                return Err((
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "Upload is too big to be processed by this server",
                ));
            }
        }
    };

    // Infer mimetype by magic numbers and check if it is allowed.
    // (Octet stream is used as fallback when */* is allowed, otherwise unknown types are rejected.)
    let (infer_str, infer_ext) = match infer::get(&upload_bytes) {
        Some(infer_result) => {
            // Check if the inferred MIME type is allowed
            if !mime::is_mime_allowed(
                &Mime::from_str(infer_result.mime_type()).unwrap(),
                &state.upload_allowed_mimetypes,
            ) {
                // Reject as unsupported type.
                debug!(
                    "Rejecting upload - server unsupported MIME type: {}",
                    infer_result.mime_type()
                );
                return Err((
                    StatusCode::UNSUPPORTED_MEDIA_TYPE,
                    "Your upload was rejected because uploading files of this type is not permitted",
                ));
            }
            (infer_result.mime_type(), infer_result.extension())
        }
        None => {
            // If no MIME type could be inferred, check if fallback is allowed.
            if state
                .upload_allowed_mimetypes
                .contains(&FALLBACK_ENABLED_MIME)
            {
                // Fallback to octet stream
                debug!(
                    "Could not infer upload MIME type - falling back to application/octet-stream"
                );
                (APPLICATION_OCTET_STREAM.essence_str(), "")
            } else {
                // Reject as unsupported type.
                debug!("Rejecting upload - No MIME type could be inferred from content");
                return Err((
                    StatusCode::UNSUPPORTED_MEDIA_TYPE,
                    "Your upload was rejected because the MIME type could not be determined",
                ));
            }
        }
    };

    // Store file by hash to prevent duplicating uploads.
    let filename = format!(
        "{}{}{}",
        Cryptography::hash_bytes(&upload_bytes, &state.persisted_salt)
            .unwrap()
            .get(..10)
            .unwrap(),
        if !infer_ext.is_empty() { "." } else { "" },
        infer_ext
    );

    // Generate a random base URL from the list of public base URLs.
    let res_url_base = state.public_base_urls.choose(&mut rand::rng()).ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "No public base URL configured.",
    ))?;

    match state
        .storage
        .write()
        .await
        .save_upload(&filename, &upload_bytes)
        .await
    {
        Ok(decryption_key) => {
            debug!("Successfully saved upload {filename} to storage.");
            Ok(Json(CreateUploadResponse {
                mimetype: infer_str,
                url: format!(
                    "{}://{}/upload/{}?key={}",
                    res_url_base.scheme(),
                    res_url_base.port().map_or(
                        res_url_base.host_str().unwrap().to_string(),
                        |f| format!("{}:{}", res_url_base.host_str().unwrap(), f,)
                    ),
                    filename,
                    decryption_key
                ),
                id: filename,
                key: decryption_key,
            }))
        }
        Err(err) => {
            error!("Failed to encrypting/writing file {filename}: {err:?}");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Your upload could not be completed successfully due to an internal server error",
            ))
        }
    }
}
