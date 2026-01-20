use crate::AppState;
use axum::{Json, extract::State};
use gifdex_lexicons::net_gifdex::actor::{
    ProfileView,
    get_profiles::{GetProfilesOutput, GetProfilesRequest},
};
use jacquard_axum::{ExtractXrpc, XrpcErrorResponse};
use jacquard_common::{
    types::{did::Did, string::Handle, uri::Uri},
    xrpc::GenericXrpcError,
};
use sqlx::query;

pub async fn handle_get_profiles(
    State(state): State<AppState>,
    ExtractXrpc(req): ExtractXrpc<GetProfilesRequest>,
) -> Result<Json<GetProfilesOutput<'static>>, XrpcErrorResponse<GenericXrpcError>> {
    let dids: Vec<String> = req.actors.iter().map(|d| d.to_string()).collect();
    let account = query!(
        "SELECT did, display_name, handle, avatar_blob_cid, indexed_at,
                (SELECT COUNT(*) FROM posts WHERE did = accounts.did) as \"post_count!\"
         FROM accounts 
         WHERE did = ANY($1)",
        &dids
    )
    .fetch_all(state.database.executor())
    .await
    .unwrap(); // TODO: Use Xrpc error.

    Ok(Json(GetProfilesOutput {
        profiles: account
            .into_iter()
            .map(|account| {
                ProfileView::new()
                    .did(Did::new_owned(&account.did).unwrap()) // Assuming Did can be created from String
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
                    .build()
            })
            .collect(),
        extra_data: None,
    }))
}
