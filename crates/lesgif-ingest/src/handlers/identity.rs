use crate::{AppState, tap::TapIdentityEventData};
use sqlx::query;
use tracing::{error, info};

pub async fn handle_identity(state: &AppState, identity: &TapIdentityEventData<'_>) -> bool {
    // Completely purge data related to accounts that are deleted or takendown.
    if matches!(identity.status.as_str(), "deleted" | "takendown") {
        if let Err(err) = query!("DELETE FROM accounts WHERE did = $1", identity.did.as_str())
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
        identity.handle.as_str(),
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
