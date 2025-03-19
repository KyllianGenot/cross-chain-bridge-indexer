use dotenv::dotenv;
use ethers::prelude::*;
use ethers::providers::{Provider, Ws};
use ethers::signers::LocalWallet;
use ethers::middleware::SignerMiddleware;
use std::env;
use std::sync::Arc;
use sqlx::PgPool;
use uuid::Uuid;
use std::str::FromStr;
use crate::abi::{TokenBridge, DepositFilter};
use crate::db::Deposit;
use hex;
use std::collections::HashSet;
use chrono::Utc;

mod abi;
mod db;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenv().ok();

    // Load environment variables
    let holesky_ws_url = env::var("HOLESKY_WS_URL").expect("HOLESKY_WS_URL must be set");
    let target_chain_ws_url = env::var("TARGET_CHAIN_WS_URL").expect("TARGET_CHAIN_WS_URL must be set");
    let holesky_bridge_address: H160 = env::var("HOLESKY_BRIDGE_ADDRESS")
        .expect("HOLESKY_BRIDGE_ADDRESS must be set")
        .parse()?;
    let target_chain_bridge_address: H160 = env::var("TARGET_CHAIN_BRIDGE_ADDRESS")
        .expect("TARGET_CHAIN_BRIDGE_ADDRESS must be set")
        .parse()?;
    let holesky_test_token: H160 = env::var("HOLESKY_TEST_TOKEN")
        .expect("HOLESKY_TEST_TOKEN must be set")
        .parse()?;
    let target_chain_test_token: H160 = env::var("TARGET_CHAIN_TEST_TOKEN")
        .expect("TARGET_CHAIN_TEST_TOKEN must be set")
        .parse()?;
    let holesky_chain_id = "holesky";
    let target_chain_id = "base-sepolia";

    println!("Holesky Test Token: 0x{}", hex::encode(holesky_test_token.as_bytes()));
    println!("Target Chain Test Token: 0x{}", hex::encode(target_chain_test_token.as_bytes()));

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&database_url).await?;
    db::init_db(&pool).await?;

    // Start from the block where the contract was deployed to avoid historical duplicates
    let holesky_start_block = 3519562; // Updated to deployment block from your logs
    let target_chain_start_block = db::get_last_processed_block(&pool, target_chain_id).await?;

    let holesky_provider = Provider::<Ws>::connect(holesky_ws_url).await?;
    let target_chain_provider = Provider::<Ws>::connect(target_chain_ws_url).await?;
    let holesky_provider = Arc::new(holesky_provider);
    let target_chain_provider = Arc::new(target_chain_provider);

    let holesky_pool = pool.clone();
    let target_chain_pool = pool.clone();
    let confirmation_pool = pool.clone();
    let tx_pool = pool.clone();
    let holesky_provider_clone = holesky_provider.clone();
    let target_chain_provider_clone = target_chain_provider.clone();

    let wallet_holesky = env::var("PRIVATE_KEY")
        .expect("PRIVATE_KEY must be set")
        .parse::<LocalWallet>()?
        .with_chain_id(17000u64); // Holesky Chain ID

    let wallet_base_sepolia = env::var("PRIVATE_KEY")
        .expect("PRIVATE_KEY must be set")
        .parse::<LocalWallet>()?
        .with_chain_id(84532u64); // Base Sepolia Chain ID

    let holesky_client = Arc::new(SignerMiddleware::new(holesky_provider.clone(), wallet_holesky));
    let target_client = Arc::new(SignerMiddleware::new(target_chain_provider.clone(), wallet_base_sepolia));

    let _holesky_handle = tokio::spawn({
        let holesky_provider = holesky_provider_clone.clone();
        async move {
            let holesky_contract = TokenBridge::new(holesky_bridge_address, holesky_provider.clone());
            let event = holesky_contract.deposit_filter();
            let filter = event.filter
                .from_block(holesky_start_block)
                .topic1(holesky_test_token);
            println!("Subscribing to Deposit events from block {} on Holesky", holesky_start_block);
            let mut stream = match holesky_provider.subscribe_logs(&filter).await {
                Ok(stream) => {
                    println!("Successfully subscribed to Holesky logs");
                    stream
                }
                Err(e) => {
                    eprintln!("Error subscribing to Holesky logs: {}", e);
                    return;
                }
            };
            let mut processed_tx_hashes: HashSet<String> = HashSet::new();

            while let Some(log) = stream.next().await {
                let transaction_hash = format!("{:?}", log.transaction_hash.unwrap_or_default());
                if processed_tx_hashes.contains(&transaction_hash) {
                    println!("Skipping duplicate event - Tx Hash: {}", transaction_hash);
                    continue;
                }
                match holesky_contract.decode_event::<DepositFilter>("Deposit", log.topics, log.data) {
                    Ok(event) => {
                        println!("Decoded Holesky Deposit event at {}: {:?}", Utc::now(), event);
                        let block_number = log.block_number.unwrap_or_default().as_u64() as i64;
                        let amount_str = event.amount.to_string();
                        let nonce_str = event.nonce.to_string();
                        let deposit = Deposit {
                            deposit_id: Uuid::new_v4(),
                            chain_id: holesky_chain_id.to_string(),
                            transaction_hash: transaction_hash.clone(),
                            block_number,
                            token_address: format!("0x{}", hex::encode(event.token.as_bytes())),
                            from_address: format!("0x{}", hex::encode(event.from.as_bytes())),
                            to_address: format!("0x{}", hex::encode(event.to.as_bytes())),
                            amount: amount_str,
                            nonce: nonce_str,
                            processed: Some(false),
                            finality_confirmed: Some(false),
                            created_at: None,
                            updated_at: None,
                        };
                        if let Err(e) = db::insert_deposit(&holesky_pool, &deposit).await {
                            eprintln!("Error inserting deposit: {}", e);
                        }
                        if let Err(e) = db::update_last_processed_block(&holesky_pool, holesky_chain_id, block_number).await {
                            eprintln!("Error updating last processed block: {}", e);
                        }
                        processed_tx_hashes.insert(transaction_hash);
                    }
                    Err(e) => eprintln!("Error decoding Deposit event: {}", e),
                }
            }
        }
    });

    let _target_chain_handle = tokio::spawn({
        let target_chain_provider = target_chain_provider_clone.clone();
        async move {
            let target_chain_contract = TokenBridge::new(target_chain_bridge_address, target_chain_provider.clone());
            let event = target_chain_contract.deposit_filter();
            let filter = event.filter
                .from_block(target_chain_start_block)
                .topic1(target_chain_test_token);
            let mut stream = match target_chain_provider.subscribe_logs(&filter).await {
                Ok(stream) => stream,
                Err(e) => {
                    eprintln!("Error subscribing to target chain logs: {}", e);
                    return;
                }
            };
            let mut processed_tx_hashes: HashSet<String> = HashSet::new();

            while let Some(log) = stream.next().await {
                let transaction_hash = format!("{:?}", log.transaction_hash.unwrap_or_default());
                if processed_tx_hashes.contains(&transaction_hash) {
                    println!("Skipping duplicate event - Tx Hash: {}", transaction_hash);
                    continue;
                }
                match target_chain_contract.decode_event::<DepositFilter>("Deposit", log.topics, log.data) {
                    Ok(event) => {
                        println!("Target Chain Deposit event at {}: {:?}", Utc::now(), event);
                        let block_number = log.block_number.unwrap_or_default().as_u64() as i64;
                        let amount_str = event.amount.to_string();
                        let nonce_str = event.nonce.to_string();
                        let deposit = Deposit {
                            deposit_id: Uuid::new_v4(),
                            chain_id: target_chain_id.to_string(),
                            transaction_hash: transaction_hash.clone(),
                            block_number,
                            token_address: format!("0x{}", hex::encode(event.token.as_bytes())),
                            from_address: format!("0x{}", hex::encode(event.from.as_bytes())),
                            to_address: format!("0x{}", hex::encode(event.to.as_bytes())),
                            amount: amount_str,
                            nonce: nonce_str,
                            processed: Some(false),
                            finality_confirmed: Some(false),
                            created_at: None,
                            updated_at: None,
                        };
                        if let Err(e) = db::insert_deposit(&target_chain_pool, &deposit).await {
                            eprintln!("Error inserting deposit: {}", e);
                        }
                        if let Err(e) = db::update_last_processed_block(&target_chain_pool, target_chain_id, block_number).await {
                            eprintln!("Error updating last processed block: {}", e);
                        }
                        processed_tx_hashes.insert(transaction_hash);
                    }
                    Err(e) => eprintln!("Error decoding Deposit event: {}", e),
                }
            }
        }
    });

    let _confirmation_handle = tokio::spawn(async move {
        let confirmation_blocks = 12;

        loop {
            let holesky_block = match holesky_provider.get_block_number().await {
                Ok(block) => block.as_u64() as i64,
                Err(e) => {
                    eprintln!("Error getting Holesky block number: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                    continue;
                }
            };
            let target_block = match target_chain_provider.get_block_number().await {
                Ok(block) => block.as_u64() as i64,
                Err(e) => {
                    eprintln!("Error getting target chain block number: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                    continue;
                }
            };

            match sqlx::query(
                r#"
                UPDATE deposits
                SET finality_confirmed = TRUE, updated_at = CURRENT_TIMESTAMP
                WHERE chain_id = $1 AND block_number <= $2 AND finality_confirmed = FALSE
                "#,
            )
            .bind(holesky_chain_id)
            .bind(holesky_block - confirmation_blocks)
            .execute(&confirmation_pool)
            .await
            {
                Ok(result) => {
                    let rows_affected = result.rows_affected();
                    println!(
                        "Updated finality for {} Holesky deposits up to block {} at {}",
                        rows_affected,
                        holesky_block - confirmation_blocks,
                        Utc::now()
                    );
                }
                Err(e) => {
                    eprintln!("Error updating finality for Holesky deposits: {}", e);
                }
            }

            match sqlx::query(
                r#"
                UPDATE deposits
                SET finality_confirmed = TRUE, updated_at = CURRENT_TIMESTAMP
                WHERE chain_id = $1 AND block_number <= $2 AND finality_confirmed = FALSE
                "#,
            )
            .bind(target_chain_id)
            .bind(target_block - confirmation_blocks)
            .execute(&confirmation_pool)
            .await
            {
                Ok(result) => {
                    let rows_affected = result.rows_affected();
                    println!(
                        "Updated finality for {} Base Sepolia deposits up to block {} at {}",
                        rows_affected,
                        target_block - confirmation_blocks,
                        Utc::now()
                    );
                }
                Err(e) => {
                    eprintln!("Error updating finality for Base Sepolia deposits: {}", e);
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        }
    });

    let _tx_handle = tokio::spawn(async move {
        let holesky_contract = TokenBridge::new(holesky_bridge_address, holesky_client.clone());
        let target_contract = TokenBridge::new(target_chain_bridge_address, target_client.clone());

        loop {
            println!("Checking for unprocessed deposits at {}", Utc::now());
            let deposits = match db::get_unprocessed_deposits(&tx_pool).await {
                Ok(deposits) => deposits,
                Err(e) => {
                    eprintln!("Error fetching unprocessed deposits: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                    continue;
                }
            };

            if deposits.is_empty() {
                println!("No unprocessed deposits found at {}", Utc::now());
            } else {
                println!("Found {} unprocessed deposits at {}", deposits.len(), Utc::now());
            }

            for deposit in deposits {
                println!("Processing deposit at {}: {:?}", Utc::now(), deposit);
                let deposit_token = match H160::from_str(&deposit.token_address) {
                    Ok(addr) => addr,
                    Err(e) => {
                        eprintln!("Error parsing deposit token address: {}", e);
                        continue;
                    }
                };

                let (contract, token_to_distribute) = if deposit.chain_id == holesky_chain_id {
                    if deposit_token != holesky_test_token {
                        eprintln!("Unexpected token for Holesky deposit: {}", deposit.token_address);
                        continue;
                    }
                    (&target_contract, target_chain_test_token)
                } else if deposit.chain_id == target_chain_id {
                    if deposit_token != target_chain_test_token {
                        eprintln!("Unexpected token for Base Sepolia deposit: {}", deposit.token_address);
                        continue;
                    }
                    (&holesky_contract, holesky_test_token)
                } else {
                    eprintln!("Unknown chain_id: {}", deposit.chain_id);
                    continue;
                };

                let to_address = match H160::from_str(&deposit.to_address) {
                    Ok(addr) => addr,
                    Err(e) => {
                        eprintln!("Error parsing to address: {}", e);
                        continue;
                    }
                };

                let amount = match U256::from_dec_str(&deposit.amount) {
                    Ok(amount) => amount,
                    Err(e) => {
                        eprintln!("Error parsing amount '{}': {}", deposit.amount, e);
                        continue;
                    }
                };

                let nonce = match U256::from_dec_str(&deposit.nonce) {
                    Ok(nonce) => nonce,
                    Err(e) => {
                        eprintln!("Error parsing nonce '{}': {}", deposit.nonce, e);
                        continue;
                    }
                };

                let is_processed = match contract.processed_deposits(nonce).call().await {
                    Ok(processed) => processed,
                    Err(e) => {
                        eprintln!("Error checking processedDeposits: {}", e);
                        true // Assume processed to avoid repeated failures
                    }
                };
                if is_processed {
                    println!("Deposit with nonce {} already processed on chain at {}", nonce, Utc::now());
                    if let Err(e) = db::update_deposit_status(&tx_pool, deposit.deposit_id, true, true).await {
                        eprintln!("Error updating deposit status: {}", e);
                    }
                    continue;
                }

                println!("Distributing for deposit at {}: {:?}", Utc::now(), deposit);
                let call = contract.distribute(token_to_distribute, to_address, amount, nonce);
                let tx = match call.send().await {
                    Ok(tx) => tx,
                    Err(e) => {
                        eprintln!("Error sending distribute transaction: {}", e);
                        continue;
                    }
                };
                if let Err(e) = tx.await {
                    eprintln!("Error waiting for transaction confirmation: {}", e);
                    continue;
                }

                if let Err(e) = db::update_deposit_status(&tx_pool, deposit.deposit_id, true, true).await {
                    eprintln!("Error updating deposit status: {}", e);
                } else {
                    println!("Successfully distributed tokens for deposit at {}: {:?}", Utc::now(), deposit.deposit_id);
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        }
    });

    tokio::signal::ctrl_c().await?;
    println!("Shutting down...");
    Ok(())
}