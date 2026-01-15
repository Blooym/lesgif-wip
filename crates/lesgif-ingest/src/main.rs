mod database;

use crate::database::Database;
use anyhow::Result;
use base64::Engine;
use chrono::DateTime;
use clap::Parser;
use dotenvy::dotenv;
use futures::{SinkExt, StreamExt};
use jacquard_common::types::{
    aturi::AtUri,
    cid::Cid,
    collection::Collection,
    did::Did,
    nsid::Nsid,
    string::{AtprotoStr, Rkey},
    tid::Tid,
};
use lesgif_lexicons::net_dollware::lesgif;
use serde::Deserialize;
use sqlx::query;
use std::{sync::Arc, time::Duration};
use tokio::sync::Semaphore;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message, client::IntoClientRequest, http::HeaderValue},
};
use tracing::{debug, error, info, instrument, warn};
use tracing_subscriber::EnvFilter;
use url::Url;

#[derive(Parser)]
#[command(author, version, about, long_about)]
struct Arguments {
    /// The local database URL to use for persistent storage.
    #[clap(long = "database-url", env = "DATABASE_URL")]
    database_url: String,

    #[clap(long = "tap-url", env = "LESGIF_INGEST_TAP_URL")]
    tap_url: Url,

    #[clap(long = "tap-password", env = "LESGIF_INGEST_TAP_PASSWORD")]
    tap_password: String,

    #[clap(long = "moderation-account", env = "LESGIF_INGEST_MODERATION_ACCOUNT")]
    moderation_account_did: Did<'static>,
}

#[derive(Clone)]
struct AppState {
    database: Database,
    moderation_account_did: Did<'static>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum TapIngestPayload<'a> {
    Record {
        id: usize,
        #[serde(borrow)]
        record: Box<RecordData<'a>>,
    },
    Identity {
        id: usize,
        #[serde(borrow)]
        identity: IdentityData<'a>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action")]
pub enum RecordAction<'a> {
    Create {
        record: AtprotoRecord<'a>,
        #[serde(borrow)]
        cid: Cid<'a>,
    },
    Update {
        record: AtprotoRecord<'a>,
        #[serde(borrow)]
        cid: Cid<'a>,
    },
    Delete,
}

#[derive(Debug, Deserialize)]
pub struct RecordData<'a> {
    pub live: bool,
    #[serde(borrow)]
    pub did: Did<'a>,
    pub rev: String,
    #[serde(borrow)]
    pub collection: Nsid<'a>,
    pub rkey: Rkey<'a>,
    #[serde(flatten, borrow)]
    pub action: RecordAction<'a>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "$type")]
pub enum AtprotoRecord<'a> {
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

#[derive(Debug, Deserialize)]
pub struct IdentityData<'a> {
    #[serde(borrow)]
    pub did: Did<'a>,
    pub handle: String,
    pub is_active: bool,
    pub status: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new("info")))
        .init();
    let args = Arguments::parse();
    let database = Database::new(&args.database_url).await?;
    let state = Arc::new(AppState {
        database,
        moderation_account_did: args.moderation_account_did,
    });
    run_tap_consumer(state, &args.tap_url, &args.tap_password).await;
    Ok(())
}

async fn run_tap_consumer(state: Arc<AppState>, tap_url: &Url, tap_password: &str) {
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
    request.headers_mut().insert(
        "Authorization",
        HeaderValue::from_str(&format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(format!("admin:{}", tap_password))
        ))?,
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
                    let payload = serde_json::from_str::<TapIngestPayload>(&text).unwrap();
                    let event_id = match &payload {
                        TapIngestPayload::Record { id, .. } => *id,
                        TapIngestPayload::Identity { id, .. } => *id,
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
            TapIngestPayload::Identity { .. } => "identity",
            TapIngestPayload::Record { .. } => "record",
        },
        did = %match &payload {
            TapIngestPayload::Identity { identity, .. } => identity.did.as_str(),
            TapIngestPayload::Record { record, .. } => record.did.as_str(),
        },
        handle = match &payload {
            TapIngestPayload::Identity { identity, .. } => Some(identity.handle.as_str()),
            TapIngestPayload::Record { .. } =>  None,
        },
        status = match &payload {
            TapIngestPayload::Identity { identity, .. } => Some(identity.status.as_str()),
            TapIngestPayload::Record { .. } => None,
        },
        is_active = match &payload {
            TapIngestPayload::Identity { identity, .. } => Some(identity.is_active),
            TapIngestPayload::Record { .. } => None,
        },
        collection = match &payload {
            TapIngestPayload::Record { record, .. } => Some(record.collection.as_str()),
            TapIngestPayload::Identity { .. } => None,
        },
        rkey = match &payload {
            TapIngestPayload::Record { record, .. } => Some(record.rkey.as_str()),
            TapIngestPayload::Identity { .. } => None,
        },
        live = match &payload {
            TapIngestPayload::Record { record, .. } => Some(record.live),
            TapIngestPayload::Identity { .. } => None,
        },
        action = match &payload {
            TapIngestPayload::Record { record, .. } => Some(match &record.action {
                RecordAction::Create { .. } => "create",
                RecordAction::Update { .. } => "update",
                RecordAction::Delete => "delete",
            }),
            TapIngestPayload::Identity { .. } => None,
        },
    )
)]
async fn process_event<'a>(state: &AppState, payload: TapIngestPayload<'a>) -> bool {
    match payload {
        TapIngestPayload::Identity { identity, .. } => {
            // Completely purge data related to accounts that are deleted or takendown.
            // TODO: Ensure this keeps moderation overlay records but deletes user labels and such.
            if matches!(identity.status.as_str(), "deleted" | "takendown") {
                if let Err(err) =
                    query!("DELETE FROM accounts WHERE did = $1", identity.did.as_str())
                        .execute(state.database.executor())
                        .await
                {
                    error!("Failed to delete account: {err:?}");
                    return false;
                };
                info!("Removed all userdata for account as it was deleted or takendown");
                return true;
            }

            // Update state of account incase of handle/status/is_active updates.
            match query!(
                "INSERT INTO accounts (did, handle, is_active, status) \
                 VALUES ($1, $2, $3, $4) \
                 ON CONFLICT(did) DO UPDATE SET \
                 handle = excluded.handle, \
                 is_active = excluded.is_active, \
                 status = excluded.status",
                identity.did.as_str(),
                identity.handle,
                identity.is_active,
                identity.status
            )
            .execute(state.database.executor())
            .await
            {
                Ok(_) => {
                    info!("Upserted stored account data into database");
                    true
                }
                Err(err) => {
                    error!("Failed to upsert account data into database: {err:?}");
                    false
                }
            }
        }
        TapIngestPayload::Record {
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
                RecordAction::Create {
                    record: ref record_payload,
                    cid: _,
                }
                | RecordAction::Update {
                    record: ref record_payload,
                    cid: _,
                } => match record_payload {
                    AtprotoRecord::LesgifFeedPost(data) => {
                        // Validate rkey format as tid:cid and matches blob
                        match record_data.rkey.split_once(":") {
                            Some((tid_str, cid_str)) => {
                                if Tid::new(tid_str).is_err() {
                                    warn!("Rejected record: invalid TID in rkey");
                                    return true;
                                }
                                let cid = match Cid::new(cid_str.as_bytes()) {
                                    Ok(cid) => cid,
                                    Err(_) => {
                                        warn!("Rejected record: invalid CID in rkey");
                                        return true;
                                    }
                                };
                                // Validate rkey CID matches blob CID
                                if cid != *data.gif.blob.blob().cid() {
                                    warn!("Rejected record: rkey CID doesn't match blob CID");
                                    return true;
                                }
                            }
                            None => {
                                warn!("Rejected record: rkey doesn't match tid:cid format");
                                return true;
                            }
                        };

                        // Validate the provided blob at least declared to be a gif.
                        if data.gif.blob.blob().mime_type.as_str() != "image/gif" {
                            warn!("Rejected record: blob isn't of mimeType 'image/gif'");
                            return true;
                        }

                        let tags_array: Option<Vec<String>> = if data.tags.is_empty() {
                            None
                        } else {
                            Some(data.tags.iter().map(|cow| cow.to_string()).collect())
                        };
                        // https://tangled.org/nonbinary.computer/jacquard/issues/27
                        match query!(
                            "INSERT INTO posts (did, rkey, blob_cid, blob_mime_type, title, blob_alt_text, \
                             tags, created_at, ingested_at) \
                             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, extract(epoch from now())::BIGINT) \
                             ON CONFLICT(did, rkey) DO UPDATE SET \
                             title = excluded.title, \
                             blob_alt_text = excluded.blob_alt_text, \
                             tags = excluded.tags, \
                             edited_at = extract(epoch from now())::BIGINT",
                            record_data.did.as_str(),
                            record_data.rkey.as_str(),
                            data.gif.blob.blob().cid().as_str(),
                            data.gif.blob.blob().mime_type.as_str(),
                            data.title.as_str(),
                            data.gif.alt.as_ref().map(|v| v.as_str()),
                            tags_array.as_deref(),
                            DateTime::parse_from_rfc3339(data.created_at.as_str())
                                .unwrap()
                                .timestamp()
                        )
                        .execute(state.database.executor())
                        .await
                        {
                            Ok(_) => {
                                info!("Upserted post into database");
                                true
                            }
                            Err(err) => {
                                error!("Failed to upsert post into database: {err:?}");
                                false
                            }
                        }
                    }
                    AtprotoRecord::LesgifFeedFavourite(data) => {
                        // NOTE: Manual string parsing is done because jacquard doesn't seem to ever return
                        // AtprotoStr::AtUri when parsing the data. Probably an upstream bug?
                        let (subject_did, _subject_collection, subject_rkey) = match &data.subject {
                            jacquard_common::types::value::Data::String(AtprotoStr::AtUri(
                                at_uri,
                            )) => (
                                at_uri.authority().to_string(),
                                at_uri.collection().unwrap().to_string(),
                                at_uri.rkey().unwrap().0.to_string(),
                            ),
                            jacquard_common::types::value::Data::String(AtprotoStr::String(s)) => {
                                debug!(
                                    "Using fallback string AtUri parser as Jacquard did not auto-set the AtUri type"
                                );
                                match AtUri::new(s.as_ref()) {
                                    Ok(at_uri) => (
                                        at_uri.authority().to_string(),
                                        at_uri
                                            .collection()
                                            .ok_or_else(|| {
                                                warn!("AtUri missing collection");
                                            })
                                            .unwrap()
                                            .to_string(),
                                        at_uri
                                            .rkey()
                                            .ok_or_else(|| {
                                                warn!("AtUri missing rkey");
                                            })
                                            .unwrap()
                                            .0
                                            .to_string(),
                                    ),
                                    Err(e) => {
                                        warn!("Failed to parse subject as AtUri: {e}");
                                        return true;
                                    }
                                }
                            }
                            _ => {
                                warn!(
                                    "Rejected record: subject {:?} is not a valid AtUri",
                                    data.subject
                                );
                                return true;
                            }
                        };
                        // https://tangled.org/nonbinary.computer/jacquard/issues/27
                        match query!(
                            "INSERT INTO post_favourites (did, rkey, post_did, \
                             post_rkey, created_at, ingested_at) \
                             VALUES ($1, $2, $3, $4, $5, extract(epoch from now())::BIGINT) \
                             ON CONFLICT (did, rkey) DO NOTHING",
                            record_data.did.as_str(),
                            record_data.rkey.as_str(),
                            subject_did.as_str(),
                            subject_rkey.as_str(),
                            DateTime::parse_from_rfc3339(data.created_at.as_str())
                                .unwrap()
                                .timestamp()
                        )
                        .execute(state.database.executor())
                        .await
                        {
                            Ok(_) => {
                                info!("Upserted feed post favourite into database");
                                true
                            }
                            Err(err) => {
                                error!(
                                    "Failed to upsert feed post favourite into database: {err:?}"
                                );
                                false
                            }
                        }
                    }
                    AtprotoRecord::LesgifActorProfile(data) => {
                        if record_data.rkey.as_str() != "self" {
                            warn!(
                                "Rejected record: actor profile record is invalid as it does not use the rkey 'self'"
                            );
                            return true;
                        }
                        match query!(
                            "INSERT INTO accounts (did, display_name, description, pronouns, \
                             avatar_blob_cid) \
                             VALUES ($1, $2, $3, $4, $5) \
                             ON CONFLICT(did) DO UPDATE SET \
                             display_name = excluded.display_name, \
                             description = excluded.description, \
                             pronouns = excluded.pronouns, \
                             avatar_blob_cid = excluded.avatar_blob_cid",
                            record_data.did.as_str(),
                            data.display_name.as_deref(),
                            data.description.as_deref(),
                            data.pronouns.as_deref(),
                            data.avatar.as_ref().map(|s| s.blob().cid().as_str())
                        )
                        .execute(state.database.executor())
                        .await
                        {
                            Ok(_) => {
                                info!("Upserted user-defined actor profile fields into database");
                                true
                            }
                            Err(err) => {
                                error!(
                                    "Failed to upsert user-defined actor profile fields into database: {err:?}"
                                );
                                false
                            }
                        }
                    }
                    AtprotoRecord::LesgifModerationLabel(data) => {
                        // https://tangled.org/nonbinary.computer/jacquard/issues/27
                        match query!(
                            "INSERT INTO labels (subject, rkey, value, reason, actor, \
                             expires_at, created_at, ingested_at) \
                             VALUES ($1, $2, $3, $4, $5, $6, $7, \
                             extract(epoch from now())::BIGINT) \
                             ON CONFLICT(subject, rkey) DO UPDATE SET \
                             reason = excluded.reason, \
                             actor = excluded.actor, \
                             value = excluded.value, \
                             expires_at = excluded.expires_at",
                            data.subject.as_str(),
                            record_data.rkey.as_str(),
                            data.value.as_str(),
                            data.reason.as_deref(),
                            record_data.did.as_str(),
                            data.expires_at.as_ref().map(|d| {
                                DateTime::parse_from_rfc3339(d.as_str())
                                    .unwrap()
                                    .timestamp()
                            }),
                            DateTime::parse_from_rfc3339(data.created_at.as_str())
                                .unwrap()
                                .timestamp()
                        )
                        .execute(state.database.executor())
                        .await
                        {
                            Ok(_) => {
                                info!("Upserted moderation label into database");
                                true
                            }
                            Err(err) => {
                                error!("Failed to upsert moderation label into database: {err:?}");
                                false
                            }
                        }
                    }
                    AtprotoRecord::Unknown => {
                        error!(
                            "No handler for record data: {record_data:?}\nIf tap is configured correctly then this message is considered a bug."
                        );
                        false
                    }
                },
                RecordAction::Delete => match record_data.collection.as_str() {
                    lesgif::feed::post::Post::NSID => {
                        match query!(
                            "DELETE FROM posts WHERE did = $1 AND rkey = $2",
                            record_data.did.as_str(),
                            record_data.rkey.as_str()
                        )
                        .execute(state.database.executor())
                        .await
                        {
                            Ok(_) => {
                                info!("Deleted post from database");
                                true
                            }
                            Err(err) => {
                                error!("Failed to delete post from database: {err:?}");
                                false
                            }
                        }
                    }
                    lesgif::feed::favourite::Favourite::NSID => {
                        match query!(
                            "DELETE FROM post_favourites WHERE did = $1 AND rkey = $2",
                            record_data.did.as_str(),
                            record_data.rkey.as_str()
                        )
                        .execute(state.database.executor())
                        .await
                        {
                            Ok(_) => {
                                info!("Deleted post favourite from database");
                                true
                            }
                            Err(err) => {
                                error!("Failed to delete post favourite from database: {err:?}");
                                false
                            }
                        }
                    }
                    lesgif::actor::profile::Profile::NSID => {
                        match query!(
                            "UPDATE accounts SET \
                             display_name = NULL, \
                             description = NULL, \
                             pronouns = NULL, \
                             avatar_blob_cid = NULL \
                             WHERE did = $1",
                            record_data.did.as_str()
                        )
                        .execute(state.database.executor())
                        .await
                        {
                            Ok(_) => {
                                info!(
                                    "Cleared all user-defined actor profile fields from database"
                                );
                                true
                            }
                            Err(err) => {
                                error!(
                                    "Failed to clear user-defined actor profile fields from database: {err:?}"
                                );
                                false
                            }
                        }
                    }
                    lesgif::moderation::label::Label::NSID => {
                        match query!(
                            "DELETE FROM labels WHERE rkey = $1",
                            record_data.rkey.as_str()
                        )
                        .execute(state.database.executor())
                        .await
                        {
                            Ok(_) => {
                                info!("Deleted moderation label from database");
                                true
                            }
                            Err(err) => {
                                error!("Failed to delete moderation label from database: {err:?}");
                                false
                            }
                        }
                    }
                    _ => {
                        error!(
                            "No handler for record data: {record_data:?}\nIf tap is configured correctly then this message is considered a bug."
                        );
                        false
                    }
                },
            }
        }
    }
}
