use crate::{AppState, MAX_BLOB_SIZE, routes::stream_with_limit};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{Response, StatusCode},
    response::IntoResponse,
};
use cid::Cid;
use floodgate::extern_types::{did::Did, tid::Tid};
use multihash_codetable::{Code, MultihashDigest};
use sqlx::query;
use std::sync::Arc;
use tracing::warn;

pub async fn get_gif_handler(
    Path((did, rkey)): Path<(String, String)>,
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

    // Parse and validate rkey (format: tid:cid)
    let rkey_cid = match rkey.split_once(':') {
        Some((tid, cid)) => {
            if Tid::new(tid).is_err() {
                warn!("invalid TID in rkey");
                return (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "Invalid or unprocessable rkey",
                )
                    .into_response();
            }
            match Cid::try_from(cid) {
                Ok(cid) => cid,
                Err(err) => {
                    warn!("invalid CID in rkey: {err:?}");
                    return (
                        StatusCode::UNPROCESSABLE_ENTITY,
                        "Invalid or unprocessable rkey",
                    )
                        .into_response();
                }
            }
        }
        None => {
            warn!("malformed rkey (expected tid:cid format)");
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                "Invalid or unprocessable rkey",
            )
                .into_response();
        }
    };

    // Ensure the post exists in our records.
    match query!(
        "SELECT EXISTS(SELECT 1 FROM posts WHERE did = $1 AND rkey = $2)",
        did.as_str(),
        rkey
    )
    .fetch_optional(state.database.executor())
    .await
    {
        Ok(result) if result.is_none() => {
            return (StatusCode::NOT_FOUND, "Post not found in records").into_response();
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
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                "DID resolution failed - identity resolver may be temporarily unavailable.",
            )
                .into_response();
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
        url.set_query(Some(&format!("did={did}&cid={rkey_cid}")));
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
    let bytes = match stream_with_limit(response, MAX_BLOB_SIZE).await {
        Ok(bytes) => bytes,
        Err(status) => return status.into_response(),
    };

    // Strictly validate the blob, computing and comparing its CID hash and validating its mime-type.
    let computed_cid = match compute_cid(&rkey_cid, &bytes) {
        Ok(cid) => cid,
        Err(code) => {
            warn!("unsupported hash algorithm: 0x{:#x}", code);
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                "Unsupported CID hash algorithm",
            )
                .into_response();
        }
    };
    if computed_cid != rkey_cid {
        warn!("CID mismatch: expected {rkey_cid}, computed {computed_cid}");
        return StatusCode::BAD_GATEWAY.into_response();
    }
    let mime_type = match infer::get(&bytes).map(|t| t.mime_type()) {
        Some(m) if matches!(m, "image/gif" | "image/webp") => m,
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

fn compute_cid(cid: &Cid, data: &[u8]) -> Result<Cid, u64> {
    match cid.hash().code() {
        0x12 => Ok(Cid::new_v1(0x55, Code::Sha2_256.digest(data))),
        code => Err(code),
    }
}
