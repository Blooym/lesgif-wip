use crate::AppState;
use axum::{Json, extract::State};
use gifdex_lexicons::net_gifdex::actor::{
    ProfileView,
    get_profile::{GetProfileError, GetProfileOutput, GetProfileRequest},
};
use jacquard_axum::{ExtractXrpc, XrpcErrorResponse};
use jacquard_common::{
    types::{string::Handle, uri::Uri},
    xrpc::XrpcError,
};
use sqlx::query;

pub async fn handle_get_profile(
    State(state): State<AppState>,
    ExtractXrpc(req): ExtractXrpc<GetProfileRequest>,
) -> Result<Json<GetProfileOutput<'static>>, XrpcErrorResponse<GetProfileError<'static>>> {
    let account = query!(
        "SELECT did, display_name, handle, avatar_blob_cid, indexed_at,
        (SELECT COUNT(*) FROM posts WHERE did = accounts.did) as \"post_count!\"
        FROM accounts WHERE did = $1",
        req.actor.as_str()
    )
    .fetch_optional(state.database.executor())
    .await
    .unwrap(); // TODO: Use Xrpc error.

    let Some(account) = account else {
        return Err(XrpcError::Xrpc(GetProfileError::ProfileNotFound(None)).into());
    };

    Ok(Json(GetProfileOutput {
        value: ProfileView::new()
            .did(req.actor)
            .handle(
                account
                    .handle
                    .map(|handle| Handle::new_owned(handle).unwrap()),
            )
            .display_name(account.display_name.map(|s| s.into()))
            .avatar(account.avatar_blob_cid.map(|blob_cid| {
                Uri::new_owned(
                    state
                        .cdn_url
                        .join(&format!("/avatar/{}/{}", account.did, blob_cid))
                        .unwrap(),
                )
                .unwrap()
            }))
            .post_count(account.post_count)
            .build(),
        extra_data: None,
    }))
}
