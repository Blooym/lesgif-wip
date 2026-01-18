use axum::body::Bytes;
use futures::StreamExt;
use reqwest::StatusCode;

pub mod avatar;
pub mod gif;

async fn stream_with_limit(
    response: reqwest::Response,
    max_size: usize,
) -> Result<Bytes, StatusCode> {
    let mut buffer = Vec::with_capacity(
        response
            .content_length()
            .map(|len| len.min(max_size as u64) as usize)
            .unwrap_or(0),
    );
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(err) => {
                tracing::warn!("error reading blob stream: {err:?}");
                return Err(StatusCode::BAD_GATEWAY);
            }
        };

        if buffer.len() + chunk.len() > max_size {
            tracing::warn!("blob exceeds size limit of {max_size} bytes");
            return Err(StatusCode::PAYLOAD_TOO_LARGE);
        }

        buffer.extend_from_slice(&chunk);
    }

    Ok(Bytes::from(buffer))
}
