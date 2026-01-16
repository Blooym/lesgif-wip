use crate::{
    AppState,
    handlers::{
        actor::{handle_profile_create_event, handle_profile_delete_event},
        feed::{
            handle_favourite_create_event, handle_favourite_delete_event, handle_post_create,
            handle_post_delete,
        },
        handle_unknown_event,
        identity::handle_identity,
        moderation::{handle_label_create_event, handle_label_delete_event},
    },
};
use anyhow::Result;
use base64::Engine;
use futures::{SinkExt, StreamExt};
use jacquard_common::types::{
    cid::Cid,
    collection::Collection,
    did::Did,
    nsid::Nsid,
    string::{Handle, Rkey},
    tid::Tid,
};
use lesgif_lexicons::net_dollware::lesgif;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message, client::IntoClientRequest, http::HeaderValue},
};
use tracing::{error, info, instrument, warn};
use url::Url;

// TODO: Extract tap client and functionality into standalone library.

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
#[non_exhaustive]
pub enum TapEvent<'a> {
    Record {
        id: usize,
        #[serde(borrow)]
        record: Box<TapRecordEventData<'a>>,
    },
    Identity {
        id: usize,
        #[serde(borrow)]
        identity: TapIdentityEventData<'a>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "lowercase")]
#[non_exhaustive]
pub enum TapRecordAction<'a> {
    Create {
        record: TapRecordData<'a>,
        #[serde(borrow)]
        cid: Cid<'a>,
    },
    Update {
        record: TapRecordData<'a>,
        #[serde(borrow)]
        cid: Cid<'a>,
    },
    Delete,
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct TapRecordEventData<'a> {
    pub live: bool,
    #[serde(borrow)]
    pub did: Did<'a>,
    pub rev: Tid,
    #[serde(borrow)]
    pub collection: Nsid<'a>,
    pub rkey: Rkey<'a>,
    #[serde(flatten, borrow)]
    pub action: TapRecordAction<'a>,
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct TapIdentityEventData<'a> {
    #[serde(borrow)]
    pub did: Did<'a>,
    pub handle: Handle<'a>,
    pub is_active: bool,
    pub status: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "$type")]
pub enum TapRecordData<'a> {
    #[serde(rename = "net.dollware.lesgif.feed.post")]
    LesgifFeedPost(#[serde(borrow)] lesgif::feed::post::Post<'a>),
    #[serde(rename = "net.dollware.lesgif.feed.favourite")]
    LesgifFeedFavourite(lesgif::feed::favourite::Favourite<'a>),
    #[serde(rename = "net.dollware.lesgif.actor.profile")]
    LesgifActorProfile(lesgif::actor::profile::Profile<'a>),
    #[serde(rename = "net.dollware.lesgif.moderation.label")]
    LesgifModerationLabel(lesgif::moderation::label::Label<'a>),
    #[serde(other)]
    Unknown,
}

pub async fn run_tap_consumer(state: Arc<AppState>, tap_url: &Url, tap_password: &str) {
    loop {
        match consume_tap(state.clone(), tap_url, tap_password).await {
            Ok(_) => info!("TAP connection closed normally"),
            Err(e) => error!("TAP error: {e}"),
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn consume_tap(state: Arc<AppState>, tap_url: &Url, tap_password: &str) -> Result<()> {
    let mut request = tap_url.as_str().into_client_request()?;
    let headers = request.headers_mut();
    headers.insert(
        "Authorization",
        HeaderValue::from_str(&format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(format!("admin:{}", tap_password))
        ))?,
    );
    headers.insert(
        "User-Agent",
        HeaderValue::from_static(concat!(
            env!("CARGO_PKG_NAME"),
            "/",
            env!("CARGO_PKG_VERSION")
        )),
    );
    let (ws_stream, _) = connect_async(request).await?;
    let (mut write, mut read) = ws_stream.split();

    let semaphore = Arc::new(Semaphore::new(50));
    while let Some(msg) = read.next().await {
        match msg? {
            Message::Text(text) => {
                let permit = semaphore.clone().acquire_owned().await?;
                let state = state.clone();
                let (should_ack, event_id) = tokio::spawn(async move {
                    let payload = serde_json::from_str::<TapEvent>(&text).unwrap();
                    let event_id = match &payload {
                        TapEvent::Record { id, .. } => *id,
                        TapEvent::Identity { id, .. } => *id,
                    };
                    let should_ack = process_event(&state, payload).await;
                    drop(permit);
                    (should_ack, event_id)
                })
                .await?;
                if should_ack {
                    let ack = serde_json::json!({
                        "type": "ack",
                        "id": event_id
                    });
                    write.send(Message::Text(ack.to_string().into())).await?;
                }
            }
            Message::Ping(data) => {
                write.send(Message::Pong(data)).await?;
            }
            Message::Close(_) => break,
            _ => continue,
        }
    }

    Ok(())
}

#[instrument(
    skip(state, payload),
    fields(
        event_type = match &payload {
            TapEvent::Identity { .. } => "identity",
            TapEvent::Record { .. } => "record",
        },
        did = %match &payload {
            TapEvent::Identity { identity, .. } => identity.did.as_str(),
            TapEvent::Record { record, .. } => record.did.as_str(),
        },
        handle = match &payload {
            TapEvent::Identity { identity, .. } => Some(identity.handle.as_str()),
            TapEvent::Record { .. } =>  None,
        },
        status = match &payload {
            TapEvent::Identity { identity, .. } => Some(identity.status.as_str()),
            TapEvent::Record { .. } => None,
        },
        is_active = match &payload {
            TapEvent::Identity { identity, .. } => Some(identity.is_active),
            TapEvent::Record { .. } => None,
        },
        collection = match &payload {
            TapEvent::Record { record, .. } => Some(record.collection.as_str()),
            TapEvent::Identity { .. } => None,
        },
        rkey = match &payload {
            TapEvent::Record { record, .. } => Some(record.rkey.as_str()),
            TapEvent::Identity { .. } => None,
        },
        live = match &payload {
            TapEvent::Record { record, .. } => Some(record.live),
            TapEvent::Identity { .. } => None,
        },
        action = match &payload {
            TapEvent::Record { record, .. } => Some(match &record.action {
                TapRecordAction::Create { .. } => "create",
                TapRecordAction::Update { .. } => "update",
                TapRecordAction::Delete => "delete",
            }),
            TapEvent::Identity { .. } => None,
        },
    )
)]
async fn process_event<'a>(state: &AppState, payload: TapEvent<'a>) -> bool {
    match payload {
        TapEvent::Identity { identity, .. } => handle_identity(state, &identity).await,
        TapEvent::Record {
            record: record_data,
            ..
        } => {
            if record_data
                .collection
                .starts_with("net.dollware.lesgif.moderation")
                && record_data.did != state.moderation_account_did
            {
                warn!(
                    "Rejected record: moderation record from account not marked as an accepted moderation account"
                );
                return true;
            }

            match record_data.action {
                TapRecordAction::Create {
                    record: ref record_payload,
                    cid: _,
                }
                | TapRecordAction::Update {
                    record: ref record_payload,
                    cid: _,
                } => match record_payload {
                    TapRecordData::LesgifFeedPost(data) => {
                        handle_post_create(state, &record_data, data).await
                    }
                    TapRecordData::LesgifFeedFavourite(data) => {
                        handle_favourite_create_event(state, &record_data, data).await
                    }
                    TapRecordData::LesgifActorProfile(data) => {
                        handle_profile_create_event(state, &record_data, data).await
                    }
                    TapRecordData::LesgifModerationLabel(data) => {
                        handle_label_create_event(state, &record_data, data).await
                    }
                    TapRecordData::Unknown => handle_unknown_event(state, &record_data).await,
                },
                TapRecordAction::Delete => match record_data.collection.as_str() {
                    lesgif::feed::post::Post::NSID => handle_post_delete(state, &record_data).await,
                    lesgif::feed::favourite::Favourite::NSID => {
                        handle_favourite_delete_event(state, &record_data).await
                    }
                    lesgif::actor::profile::Profile::NSID => {
                        handle_profile_delete_event(state, &record_data).await
                    }
                    lesgif::moderation::label::Label::NSID => {
                        handle_label_delete_event(state, &record_data).await
                    }
                    _ => handle_unknown_event(state, &record_data).await,
                },
            }
        }
    }
}
