use chrono::{DateTime, Utc};
use sqlx::{Error, PgPool, FromRow};
use uuid::Uuid;
use ethers::types::U256;

#[derive(Debug, Clone, FromRow)]
pub struct Deposit {
    pub deposit_id: Uuid,
    pub chain_id: String,
    pub transaction_hash: String,
    pub block_number: i64,
    pub token_address: String,
    pub from_address: String,
    pub to_address: String,
    pub amount: String,
    pub nonce: String,
    pub processed: Option<bool>,
    pub finality_confirmed: Option<bool>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct LastProcessedBlock {
    pub chain_id: String,
    pub last_block: i64,
}

pub async fn init_db(pool: &PgPool) -> Result<(), Error> {
    // Drop the distributions table if it exists
    sqlx::query("DROP TABLE IF EXISTS distributions")
        .execute(pool)
        .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS deposits (
            deposit_id UUID PRIMARY KEY,
            chain_id TEXT NOT NULL,
            transaction_hash TEXT NOT NULL,
            block_number BIGINT NOT NULL,
            token_address TEXT NOT NULL,
            from_address TEXT NOT NULL,
            to_address TEXT NOT NULL,
            amount TEXT NOT NULL,
            nonce TEXT NOT NULL,
            processed BOOLEAN DEFAULT FALSE,
            finality_confirmed BOOLEAN DEFAULT FALSE,
            created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
            CONSTRAINT unique_nonce_chain UNIQUE (nonce, chain_id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS last_processed_blocks (
            chain_id TEXT PRIMARY KEY,
            last_block BIGINT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn insert_deposit(pool: &PgPool, deposit: &Deposit) -> Result<(), Error> {
    if let Err(e) = U256::from_dec_str(&deposit.nonce) {
        eprintln!("Invalid nonce for deposit: {} (error: {})", deposit.nonce, e);
        return Ok(());
    }
    match sqlx::query(
        r#"
        INSERT INTO deposits (
            deposit_id, chain_id, transaction_hash, block_number, token_address,
            from_address, to_address, amount, nonce, processed, finality_confirmed,
            created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        "#,
    )
    .bind(deposit.deposit_id)
    .bind(&deposit.chain_id)
    .bind(&deposit.transaction_hash)
    .bind(deposit.block_number)
    .bind(&deposit.token_address)
    .bind(&deposit.from_address)
    .bind(&deposit.to_address)
    .bind(&deposit.amount)
    .bind(&deposit.nonce)
    .bind(deposit.processed.unwrap_or(false))
    .bind(deposit.finality_confirmed.unwrap_or(false))
    .bind(deposit.created_at.unwrap_or_else(Utc::now))
    .bind(deposit.updated_at.unwrap_or_else(Utc::now))
    .execute(pool)
    .await {
        Ok(_) => {
            println!("Deposit inserted: {:?}", deposit.deposit_id);
            Ok(())
        }
        Err(e) if e.to_string().contains("duplicate key value") => {
            println!("Skipping duplicate deposit: {} (nonce: {}, chain: {})", 
                deposit.transaction_hash, deposit.nonce, deposit.chain_id);
            Ok(())
        }
        Err(e) => Err(e),
    }
}

pub async fn get_last_processed_block(pool: &PgPool, chain_id: &str) -> Result<i64, Error> {
    let row: (i64,) = sqlx::query_as("SELECT last_block FROM last_processed_blocks WHERE chain_id = $1")
        .bind(chain_id)
        .fetch_optional(pool)
        .await?
        .unwrap_or((0,));
    Ok(row.0)
}

pub async fn update_last_processed_block(pool: &PgPool, chain_id: &str, block_number: i64) -> Result<(), Error> {
    sqlx::query(
        r#"
        INSERT INTO last_processed_blocks (chain_id, last_block)
        VALUES ($1, $2)
        ON CONFLICT (chain_id)
        DO UPDATE SET last_block = $2
        "#,
    )
    .bind(chain_id)
    .bind(block_number)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_unprocessed_deposits(pool: &PgPool) -> Result<Vec<Deposit>, Error> {
    let deposits = sqlx::query_as::<_, Deposit>(
        r#"
        SELECT * FROM deposits
        WHERE processed = FALSE AND finality_confirmed = TRUE
        "#,
    )
    .fetch_all(pool)
    .await?;
    println!("Found {} unprocessed deposits", deposits.len());
    Ok(deposits)
}

pub async fn update_deposit_status(
    pool: &PgPool,
    deposit_id: Uuid,
    processed: bool,
    finality_confirmed: bool,
) -> Result<(), Error> {
    sqlx::query(
        r#"
        UPDATE deposits
        SET processed = $2, finality_confirmed = $3, updated_at = CURRENT_TIMESTAMP
        WHERE deposit_id = $1
        "#,
    )
    .bind(deposit_id)
    .bind(processed)
    .bind(finality_confirmed)
    .execute(pool)
    .await?;
    Ok(())
}