use sqlx::{PgPool, Error};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use bigdecimal::BigDecimal;

// Struct representing a deposit record
#[derive(sqlx::FromRow, Debug)]
pub struct Deposit {
    pub deposit_id: Uuid,
    pub chain_id: String,
    pub transaction_hash: String,
    pub block_number: i64,
    pub token_address: String,
    pub from_address: String,
    pub to_address: String,
    pub amount: BigDecimal,
    pub nonce: i64,
    pub processed: Option<bool>,  // Option<bool> pour gérer NULL
    pub finality_confirmed: Option<bool>,  // Option<bool> pour gérer NULL
    pub created_at: Option<DateTime<Utc>>,  // Option<DateTime<Utc>> pour gérer NULL
    pub updated_at: Option<DateTime<Utc>>,  // Option<DateTime<Utc>> pour gérer NULL
}

#[allow(dead_code)]  // Ajouté pour supprimer l’avertissement sur les champs non utilisés
#[derive(sqlx::FromRow, Debug)]
pub struct Distribution {
    pub distribution_id: Uuid,
    pub deposit_id: Uuid,
    pub chain_id: String,
    pub transaction_hash: String,
    pub block_number: i64,
    pub token_address: String,
    pub recipient_address: String,
    pub amount: BigDecimal,
    pub nonce: i64,
    pub status: String,
    pub created_at: Option<DateTime<Utc>>,  // Option<DateTime<Utc>> pour gérer NULL
    pub updated_at: Option<DateTime<Utc>>,  // Option<DateTime<Utc>> pour gérer NULL
}

// Insert a new deposit into the database
pub async fn insert_deposit(pool: &PgPool, deposit: &Deposit) -> Result<(), Error> {
    sqlx::query!(
        r#"
        INSERT INTO deposits (
            deposit_id, chain_id, transaction_hash, block_number, token_address, 
            from_address, to_address, amount, nonce, processed, finality_confirmed, 
            created_at, updated_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        "#,
        deposit.deposit_id,
        &deposit.chain_id,
        &deposit.transaction_hash,
        deposit.block_number,
        &deposit.token_address,
        &deposit.from_address,
        &deposit.to_address,
        &deposit.amount,
        deposit.nonce,
        deposit.processed,  // sqlx gère automatiquement Option<bool>
        deposit.finality_confirmed,  // sqlx gère automatiquement Option<bool>
        deposit.created_at,  // sqlx gère automatiquement Option<DateTime<Utc>>
        deposit.updated_at  // sqlx gère automatiquement Option<DateTime<Utc>>
    )
    .execute(pool)
    .await?;
    Ok(())
}

// Fetch all unprocessed deposits with confirmed finality
pub async fn get_unprocessed_deposits(pool: &PgPool) -> Result<Vec<Deposit>, Error> {
    sqlx::query_as!(
        Deposit,
        r#"SELECT * FROM deposits WHERE COALESCE(processed, false) = false AND COALESCE(finality_confirmed, false) = true"#
    )
    .fetch_all(pool)
    .await
}

// Update deposit status (processed and finality_confirmed)
pub async fn update_deposit_status(
    pool: &PgPool,
    deposit_id: Uuid,
    processed: Option<bool>,  // Option<bool> pour correspondre à la structure
    finality_confirmed: Option<bool>,  // Option<bool> pour correspondre à la structure
) -> Result<(), Error> {
    sqlx::query!(
        r#"
        UPDATE deposits 
        SET processed = $1, finality_confirmed = $2, updated_at = COALESCE($3, NOW() AT TIME ZONE 'UTC') 
        WHERE deposit_id = $4
        "#,
        processed,  // sqlx gère automatiquement Option<bool> -> NULL si None
        finality_confirmed,  // sqlx gère automatiquement Option<bool> -> NULL si None
        Utc::now().naive_utc(),  // Convertir DateTime<Utc> en NaiveDateTime
        deposit_id
    )
    .execute(pool)
    .await?;
    Ok(())
}