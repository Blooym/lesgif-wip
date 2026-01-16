use anyhow::Result;
use sqlx::{PgExecutor, PgPool, PgTransaction, migrate, postgres::PgPoolOptions};

pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn new(connect_string: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(20)
            .connect(connect_string)
            .await?;
        migrate!("../sqlx-migrations").run(&pool).await?;
        Ok(Self { pool })
    }

    /// Obtain the executor for the database pool.
    pub fn executor(&self) -> impl PgExecutor<'_> + Copy {
        &self.pool
    }

    /// Create a new transaction using the database pool.
    pub async fn transaction(&self) -> Result<PgTransaction<'_>> {
        Ok(self.pool.begin().await?)
    }
}
