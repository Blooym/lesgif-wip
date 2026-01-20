use anyhow::Result;
use floodgate::api::RecordEventData;
use gifdex_lexicons::net_gifdex;
use sqlx::{PgTransaction, query};
use tracing::{error, info};

pub async fn handle_label_create_event(
    record_data: &RecordEventData<'_>,
    data: &net_gifdex::labeler::label::Label<'_>,
    tx: &mut PgTransaction<'_>,
) -> Result<()> {
    let (subject_did, subject_collection, subject_rkey) = (
        data.subject.authority().as_str(),
        data.subject.collection().map(|v| v.as_str()),
        data.subject.rkey().map(|v| v.0.as_str()),
    );
    let (rule_did, rule_rkey) = match (
        data.rule.authority().as_str(),
        data.rule.collection().map(|v| v.as_str()),
        data.rule.rkey().map(|v| v.0.as_str()),
    ) {
        (did, Some("net.gifdex.labeler.rule"), Some(rkey)) if did == record_data.did.as_str() => {
            (did, rkey)
        }
        (_, None, _) | (_, _, None) => {
            tracing::warn!(
                rule_uri = data.rule.as_str(),
                "Rejected record: rule must be a complete AT-URI with collection and rkey"
            );
            return Ok(());
        }
        (_, Some(collection), _) if collection != "net.gifdex.labeler.rule" => {
            tracing::warn!(
                rule_collection = collection,
                "Rejected record: rule must reference net.gifdex.labeler.rule collection"
            );
            return Ok(());
        }
        (rule_did, _, _) => {
            tracing::warn!(
                rule_did = rule_did,
                labeler_did = record_data.did.as_str(),
                "Rejected record: labeler can only apply their own rules"
            );
            return Ok(());
        }
    };

    match query!(
        "INSERT INTO labels (\
             rkey, did, rule_did, rule_rkey, \
             subject_did, subject_collection, subject_rkey, \
             reason, created_at, expires_at \
         ) VALUES ( \
             $1, $2, $3, $4, $5, $6, $7, $8, $9, $10 \
         ) \
         ON CONFLICT(did, rkey) DO UPDATE SET \
         rule_did = excluded.rule_did, \
         rule_rkey = excluded.rule_rkey, \
         subject_did = excluded.subject_did, \
         subject_collection = excluded.subject_collection, \
         subject_rkey = excluded.subject_rkey, \
         reason = excluded.reason, \
         created_at = excluded.created_at, \
         edited_at = extract(epoch from now())::BIGINT, \
         expires_at = excluded.expires_at",
        record_data.rkey.as_str(),
        record_data.did.as_str(),
        rule_did,
        rule_rkey,
        subject_did,
        subject_collection,
        subject_rkey,
        data.reason.as_deref(),
        data.created_at.as_ref().timestamp_millis(),
        data.expires_at
            .as_ref()
            .map(|expiry| expiry.as_ref().timestamp_micros())
    )
    .execute(&mut **tx)
    .await
    {
        Ok(_) => {
            info!("Upserted label application");
            Ok(())
        }
        Err(err) => {
            error!("Failed to upsert label application: {err:?}");
            Err(err.into())
        }
    }
}

pub async fn handle_label_delete_event(
    record_data: &RecordEventData<'_>,
    tx: &mut PgTransaction<'_>,
) -> Result<()> {
    match query!(
        "DELETE FROM labels WHERE did = $1 AND rkey = $2",
        record_data.did.as_str(),
        record_data.rkey.as_str()
    )
    .execute(&mut **tx)
    .await
    {
        Ok(_) => {
            info!("Deleted label application");
            Ok(())
        }
        Err(err) => {
            error!("Failed to delete label application: {err:?}");
            Err(err.into())
        }
    }
}
