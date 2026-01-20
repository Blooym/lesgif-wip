use anyhow::Result;
use floodgate::api::RecordEventData;
use gifdex_lexicons::net_gifdex;
use sqlx::{PgTransaction, query};
use tracing::{error, info, warn};

pub async fn handle_profile_create_event(
    record_data: &RecordEventData<'_>,
    data: &net_gifdex::actor::profile::Profile<'_>,
    tx: &mut PgTransaction<'_>,
) -> Result<()> {
    // Ensure the record rkey is a valid exactly 'self'.
    if record_data.rkey.as_str() != "self" {
        warn!(
            "Rejected record: actor profile record is invalid as it does not use the rkey 'self'"
        );
        return Ok(());
    }

    // Validate that the avatar blob CID is valid,
    // and that the reported mimetype + size are in bounds.
    if let Some(avatar) = &data.avatar {
        if !avatar.blob().cid().is_valid() {
            warn!("Rejected record: invalid blob CID in for avatar");
            return Ok(());
        };
        if !matches!(avatar.blob().mime_type.as_str(), "image/png" | "image/jpeg") {
            warn!("Rejected record: blob isn't a valid mimetype");
            return Ok(());
        }
        if avatar.blob().size == 3 * 1024 * 1024 {
            warn!("Rejected record: blob is above maximum size");
            return Ok(());
        }
    }

    match query!(
        "INSERT INTO accounts (did, display_name, pronouns, \
         avatar_blob_cid, created_at) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT(did) DO UPDATE SET \
         display_name = excluded.display_name, \
         pronouns = excluded.pronouns, \
         avatar_blob_cid = excluded.avatar_blob_cid, \
         created_at = excluded.created_at",
        record_data.did.as_str(),
        data.display_name.as_deref(),
        data.pronouns.as_deref(),
        data.avatar.as_ref().map(|s| s.blob().cid().as_str()),
        data.created_at.as_ref().timestamp_millis()
    )
    .execute(&mut **tx)
    .await
    {
        Ok(_) => {
            info!("Upserted user-defined actor profile fields into database");
            Ok(())
        }
        Err(err) => {
            error!("Failed to upsert user-defined actor profile fields into database: {err:?}");
            Err(err.into())
        }
    }
}

pub async fn handle_profile_delete_event(
    record_data: &RecordEventData<'_>,
    tx: &mut PgTransaction<'_>,
) -> Result<()> {
    if record_data.rkey.as_str() != "self" {
        warn!(
            "Rejected record: actor profile record is invalid as it does not use the rkey 'self'"
        );
        return Ok(());
    }
    match query!(
        "UPDATE accounts SET \
         display_name = NULL, \
         pronouns = NULL, \
         avatar_blob_cid = NULL \
         WHERE did = $1",
        record_data.did.as_str()
    )
    .execute(&mut **tx)
    .await
    {
        Ok(_) => {
            info!("Cleared all user-defined actor profile fields from database");
            Ok(())
        }
        Err(err) => {
            error!("Failed to clear user-defined actor profile fields from database: {err:?}");
            Err(err.into())
        }
    }
}
