use crate::{AppState, MAX_AVATAR_SIZE, routes::stream_with_limit};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{Response, StatusCode},
    response::IntoResponse,
};
use cid::Cid;
use floodgate::extern_types::did::Did;
use multihash_codetable::{Code, MultihashDigest};
use sqlx::query;
use std::sync::Arc;
use tracing::warn;

pub async fn get_avatar_handler(
    Path((did, cid)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // Strictly verify the received path types.
    let did = match Did::new(&did) {
        Ok(did) => did,
        Err(err) => {
            warn!("invalid DID '{did}': {err:?}");
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                "Invalid or unprocessable DID",
            )
                .into_response();
        }
    };
    let cid = match Cid::try_from(cid.as_str()) {
        Ok(cid) => cid,
        Err(err) => {
            warn!("invalid CID '{cid}': {err:?}");
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                "Invalid or unprocessable CID",
            )
                .into_response();
        }
    };

    // Ensur the CID that's being requested matches our records.
    match query!(
        "SELECT EXISTS(SELECT 1 FROM accounts WHERE did = $1 AND avatar_blob_cid = $2)",
        did.as_str(),
        cid.to_string()
    )
    .fetch_optional(state.database.executor())
    .await
    {
        Ok(result) if result.is_none() => {
            return (
                StatusCode::NOT_FOUND,
                "Blob CID does not match account record blog CID",
            )
                .into_response();
        }
        Ok(_) => {}
        Err(err) => {
            warn!("database error: {err:?}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    // Get the user's PDS URL from their DID document
    let pds_url = match state.tap_client.resolve_did(&did).await {
        Ok(doc) => match doc.pds_endpoint() {
            Some(url) => url,
            None => {
                warn!("No PDS endpoint found for {did}");
                return (
                    StatusCode::NOT_FOUND,
                    "No AtprotoPersonalDataServer service endpoint found in resolved DID document",
                )
                    .into_response();
            }
        },
        Err(err) => {
            warn!("failed to resolve DID {did}: {err:?}");
            return (StatusCode::BAD_GATEWAY, "Failed to resolve DID").into_response();
        }
    };
    let blob_url = {
        let mut url = match pds_url.join("/xrpc/com.atproto.sync.getBlob") {
            Ok(url) => url,
            Err(err) => {
                warn!("failed to build XRPC URL: {err:?}");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };
        url.set_query(Some(&format!("did={did}&cid={cid}")));
        url
    };

    // Fetch the blob from the user's PDS
    let response = match state.http_client.get(blob_url).send().await {
        Ok(resp) => resp,
        Err(err) => {
            warn!("failed to fetch blob from PDS: {err:?}");
            return (
                StatusCode::BAD_GATEWAY,
                "Failed to fetch blob from upstream PDS",
            )
                .into_response();
        }
    };
    if !response.status().is_success() {
        warn!("PDS returned error status: {}", response.status());
        return (
            StatusCode::BAD_GATEWAY,
            "Failed to fetch blob from upstream PDS",
        )
            .into_response();
    }
    let bytes = match stream_with_limit(response, MAX_AVATAR_SIZE).await {
        Ok(bytes) => bytes,
        Err(status) => return status.into_response(),
    };

    // Strictly validate the blob, computing and comparing it's CID hash and best-guessing it's mime-type.
    let computed_cid = match cid.hash().code() {
        0x12 => Cid::new_v1(0x55, Code::Sha2_256.digest(&bytes)),
        _ => {
            warn!("unsupported hash algorithm: 0x{:x}", cid.hash().code());
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                "Unsupported CID hash algorithm",
            )
                .into_response();
        }
    };
    if computed_cid != cid {
        warn!("CID mismatch: expected {cid}, computed {computed_cid}");
        return StatusCode::BAD_GATEWAY.into_response();
    }
    let mime_type = match infer::get(&bytes).map(|t| t.mime_type()) {
        Some(m) if matches!(m, "image/png" | "image/jpeg" | "image/webp") => m,
        _ => {
            warn!("invalid or unsupported image format");
            return StatusCode::UNPROCESSABLE_ENTITY.into_response();
        }
    };

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", mime_type)
        .header("Content-Security-Policy", "default-src 'none'; sandbox")
        .header("X-Content-Type-Options", "nosniff")
        .header("Cache-Control", "public, max-age=604800")
        .header(
            "Upstream-PDS",
            format!(" {}", pds_url.host_str().unwrap_or("unknown")),
        )
        .body(Body::from(bytes))
        .unwrap()
        .into_response()
}
