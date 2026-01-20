use anyhow::Result;
use floodgate::api::IdentityEventData;
use sqlx::{PgTransaction, query};
use tracing::{error, info};

pub async fn handle_identity(
    identity: &IdentityEventData<'_>,
    tx: &mut PgTransaction<'_>,
) -> Result<()> {
    // Completely purge data related to accounts that are deleted or takendown.
    // Note: this does not delete any labels applied to the account or their content.
    if matches!(identity.status.as_str(), "deleted" | "takendown") {
        if let Err(err) = query!("DELETE FROM accounts WHERE did = $1", identity.did.as_str())
            .execute(&mut **tx)
            .await
        {
            error!("Failed to delete account: {err:?}");
            return Err(err.into());
        };
        info!("Removed all userdata for account as it was deleted or takendown");
        return Ok(());
    }

    // Update state of account incase of handle/status/is_active updates.
    match query!(
        "INSERT INTO accounts (did, handle, is_active, status, created_at) \
         VALUES ($1, $2, $3, $4, (extract(epoch from now()) * 1000)::BIGINT) \
         ON CONFLICT(did) DO UPDATE SET \
         handle = excluded.handle, \
         is_active = excluded.is_active, \
         status = excluded.status",
        identity.did.as_str(),
        identity.handle.as_str(),
        identity.is_active,
        identity.status
    )
    .execute(&mut **tx)
    .await
    {
        Ok(_) => {
            info!("Upserted stored account data into database");
            Ok(())
        }
        Err(err) => {
            error!("Failed to upsert account data into database: {err:?}");
            Err(err.into())
        }
    }
}
