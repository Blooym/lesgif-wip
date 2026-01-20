use crate::AppState;
use axum::{Json, extract::State};
use gifdex_lexicons::net_gifdex::{
    actor::{
        ProfileViewBasic,
        get_posts::{GetPostsError, GetPostsOutput, GetPostsRequest},
    },
    feed::{self, PostFeedView, PostFeedViewMedia},
};
use jacquard_axum::{ExtractXrpc, XrpcErrorResponse};
use jacquard_common::{
    IntoStatic,
    chrono::{TimeZone, Utc},
    types::{aturi::AtUri, string::Handle, uri::Uri},
    xrpc::XrpcError,
};
use sqlx::query;

pub async fn handle_get_actor_posts(
    State(state): State<AppState>,
    ExtractXrpc(req): ExtractXrpc<GetPostsRequest>,
) -> Result<Json<GetPostsOutput<'static>>, XrpcErrorResponse<GetPostsError<'static>>> {
    let account = query!(
        "SELECT did, display_name, handle, avatar_blob_cid, indexed_at
        FROM accounts WHERE did = $1",
        req.actor.as_str()
    )
    .fetch_optional(state.database.executor())
    .await
    .unwrap(); // TODO: Use Xrpc error.

    let Some(account) = account else {
        return Err(XrpcError::Xrpc(GetPostsError::ActorNotFound(None)).into());
    };

    // Build the profile view
    let profile = ProfileViewBasic::new()
        .did(req.actor.clone())
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
        .build();

    // Parse cursor if provided
    // Fetch posts with pagination
    let cursor_created_at: Option<i64> = req.cursor.as_ref().and_then(|c| c.parse().ok());
    let limit = req.limit.unwrap_or(50).min(100) as i64;
    let posts = query!(
        "SELECT did, rkey, title, tags, languages, media_blob_cid, media_blob_mime, 
                media_blob_alt, created_at, edited_at, indexed_at,
                (SELECT COUNT(*) FROM post_favourites WHERE post_did = posts.did AND post_rkey = posts.rkey) as \"favourite_count!\"
         FROM posts
         WHERE did = $1 
           AND ($2::BIGINT IS NULL OR created_at < $2)
         ORDER BY created_at DESC
         LIMIT $3", req.actor.as_str(), cursor_created_at, limit
    )
    .fetch_all(state.database.executor())
    .await
    .unwrap(); // TODO: Use Xrpc error

    // Generate next cursor if we have more posts
    let cursor = if posts.len() == limit as usize {
        posts.last().map(|post| post.created_at.to_string())
    } else {
        None
    };

    // Build post views
    let post_views: Vec<PostFeedView> = posts
        .into_iter()
        .map(|post| {
            let uri = AtUri::new_owned(format!(
                "at://{}/net.gifdex.feed.post/{}",
                post.did, post.rkey
            ))
            .unwrap();
            PostFeedView::new()
                .uri(uri)
                .title(post.title.into_static())
                .tags(
                    post.tags
                        .map(|tags| tags.into_iter().map(|t| t.into()).collect()),
                )
                .languages(
                    post.languages
                        .map(|langs| langs.into_iter().map(|l| l.into()).collect()),
                )
                .media(
                    PostFeedViewMedia::new()
                        .url(
                            Uri::new_owned(
                                state
                                    .cdn_url
                                    .join(&format!("/media/{}/{}", account.did, post.rkey))
                                    .unwrap(),
                            )
                            .unwrap(),
                        )
                        .mime_type(post.media_blob_mime.into_static())
                        .alt(post.media_blob_alt.map(|s| s.into()))
                        .build(),
                )
                .favourite_count(post.favourite_count)
                .author(profile.clone())
                .viewer(feed::ViewerState::new().liked(false).build())
                .created_at(
                    Utc.timestamp_millis_opt(post.created_at)
                        .unwrap()
                        .fixed_offset(),
                )
                .indexed_at(
                    Utc.timestamp_millis_opt(post.indexed_at)
                        .unwrap()
                        .fixed_offset(),
                )
                .build()
        })
        .collect();

    Ok(Json(GetPostsOutput {
        feed: post_views,
        cursor: cursor.map(|c| c.into()),
        extra_data: None,
    }))
}
