use dotenv::dotenv;
use sqlx::PgPool;
use std::env;
use chrono::Utc;
use uuid::Uuid;
use bigdecimal::BigDecimal;
use crate::db::{Deposit, insert_deposit, get_unprocessed_deposits, update_deposit_status};

mod db;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env");
    let pool = PgPool::connect(&database_url).await?;

    println!("Database connected successfully!");

    // Generate a unique nonce based on the current timestamp in nanoseconds
    let unique_nonce = Utc::now().timestamp_nanos();

    // Test inserting a deposit with a unique transaction_hash and nonce
    let unique_transaction_hash = format!("0x{}", Uuid::new_v4().to_string().replace("-", ""));
    let test_deposit = Deposit {
        deposit_id: Uuid::new_v4(),
        chain_id: "holesky".to_string(),
        transaction_hash: unique_transaction_hash,
        block_number: 1000,
        token_address: "0x5678".to_string(),
        from_address: "0x9abc".to_string(),
        to_address: "0xdef0".to_string(),
        amount: BigDecimal::from(1000000000000000000_i64), // 1 token (18 decimals)
        nonce: unique_nonce,  // Unique nonce to avoid duplication
        processed: Some(false),
        finality_confirmed: Some(false),
        created_at: Some(Utc::now()),
        updated_at: Some(Utc::now()),
    };

    insert_deposit(&pool, &test_deposit).await?;
    println!("Test deposit inserted successfully with transaction_hash: {} and nonce: {}", test_deposit.transaction_hash, test_deposit.nonce);

    // Test fetching unprocessed deposits
    let unprocessed = get_unprocessed_deposits(&pool).await?;
    for deposit in unprocessed {
        println!("Unprocessed deposit: {:?}", deposit);
    }

    // Test updating deposit status
    update_deposit_status(&pool, test_deposit.deposit_id, Some(true), Some(true)).await?;
    println!("Deposit status updated successfully!");

    // Fetch and print updated unprocessed deposits
    let updated_unprocessed = get_unprocessed_deposits(&pool).await?;
    println!("Unprocessed deposits after update: {:?}", updated_unprocessed);

    Ok(())
}