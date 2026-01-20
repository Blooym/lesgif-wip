use anyhow::Result;
use floodgate::api::RecordEventData;
use gifdex_lexicons::net_gifdex;
use jacquard_common::types::{cid::Cid, collection::Collection, tid::Tid};
use sqlx::{PgTransaction, query};
use tracing::{error, info};

pub async fn handle_favourite_create_event(
    record_data: &RecordEventData<'_>,
    data: &net_gifdex::feed::favourite::Favourite<'_>,
    tx: &mut PgTransaction<'_>,
) -> Result<()> {
    // Ensure the record rkey is a valid TID .
    if Tid::new(&record_data.rkey).is_err() {
        tracing::warn!("Rejected record: invalid rkey");
        return Ok(());
    }
    // Ensure the record's referenced subject is a post.
    let (post_did, post_collection, post_rkey) = match (
        data.subject.authority(),
        data.subject.collection(),
        data.subject.rkey(),
    ) {
        (did, Some(collection), Some(rkey)) => {
            // Ensure post rkey is valid.
            match rkey.as_ref().split_once(":") {
                Some((tid, cid)) => {
                    if Tid::new(tid).is_err() {
                        tracing::warn!("Rejected record: invalid TID in rkey");
                        return Ok(());
                    }
                    if !Cid::str(cid).is_valid() {
                        tracing::warn!("Rejected record: invalid CID in rkey");
                        return Ok(());
                    };
                }
                None => {
                    tracing::warn!("Rejected record: rkey doesn't match tid:cid format");
                    return Ok(());
                }
            };
            (did, collection, rkey)
        }
        at_uri @ _ => {
            tracing::warn!("Rejected record: invalid subject at-uri ({at_uri:?})");
            return Ok(());
        }
    };
    if post_collection.as_str() != net_gifdex::feed::post::Post::NSID {
        tracing::warn!(
            "Rejected record: subject at-uri referenced a collection that was not {}",
            net_gifdex::feed::post::Post::NSID
        );
        return Ok(());
    }

    match query!(
        "INSERT INTO post_favourites (did, rkey, post_did, \
         post_rkey, created_at, indexed_at) \
         VALUES ($1, $2, $3, $4, $5, extract(epoch from now())::BIGINT) \
         ON CONFLICT (did, rkey) DO NOTHING",
        record_data.did.as_str(),
        record_data.rkey.as_str(),
        post_did.as_str(),
        post_rkey.as_ref(),
        data.created_at.as_ref().timestamp_millis()
    )
    .execute(&mut **tx)
    .await
    {
        Ok(_) => {
            info!("Upserted feed post favourite into database");
            Ok(())
        }
        Err(err) => {
            error!("Failed to upsert feed post favourite into database: {err:?}");
            Err(err.into())
        }
    }
}

pub async fn handle_favourite_delete_event(
    record_data: &RecordEventData<'_>,
    tx: &mut PgTransaction<'_>,
) -> Result<()> {
    match query!(
        "DELETE FROM post_favourites WHERE did = $1 AND rkey = $2",
        record_data.did.as_str(),
        record_data.rkey.as_str()
    )
    .execute(&mut **tx)
    .await
    {
        Ok(_) => {
            info!("Deleted post favourite from database");
            Ok(())
        }
        Err(err) => {
            error!("Failed to delete post favourite from database: {err:?}");
            Err(err.into())
        }
    }
}
