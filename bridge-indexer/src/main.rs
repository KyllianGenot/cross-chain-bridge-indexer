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
use tokio::time::{sleep, Duration};

mod abi;
mod db;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenv().ok();

    // Environment Setup
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

    let holesky_start_block = 3519562; // Deployment block for Holesky
    let target_chain_start_block = db::get_last_processed_block(&pool, target_chain_id).await.unwrap_or(0);

    // Provider and Client Initialization
    let holesky_provider = Provider::<Ws>::connect(holesky_ws_url).await?;
    let target_chain_provider = Provider::<Ws>::connect(target_chain_ws_url).await?;
    let holesky_provider = Arc::new(holesky_provider);
    let target_chain_provider = Arc::new(target_chain_provider);

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

    // Unpause contracts at startup
    println!("Starting indexer and unpausing contracts...");
    unpause_contracts(&holesky_client, &target_client, holesky_bridge_address, target_chain_bridge_address).await;

    let holesky_pool = pool.clone();
    let target_chain_pool = pool.clone();
    let confirmation_pool = pool.clone();
    let tx_pool = pool.clone();
    let holesky_provider_clone = holesky_provider.clone();
    let target_chain_provider_clone = target_chain_provider.clone();

    // Holesky Event Listener
    let _holesky_handle = tokio::spawn({
        let holesky_provider = holesky_provider_clone.clone();
        async move {
            let holesky_contract = TokenBridge::new(holesky_bridge_address, holesky_provider.clone());
            let event = holesky_contract.deposit_filter();
            let filter = event.filter
                .from_block(holesky_start_block)
                .topic1(holesky_test_token);
            println!("Subscribing to Deposit events from block {} on Holesky", holesky_start_block);

            let mut stream = loop {
                match holesky_provider.subscribe_logs(&filter).await {
                    Ok(stream) => {
                        println!("Successfully subscribed to Holesky logs");
                        break stream;
                    }
                    Err(e) => {
                        eprintln!("Failed to subscribe to Holesky logs: {}. Retrying in 60s...", e);
                        sleep(Duration::from_secs(60)).await;
                    }
                }
            };

            let mut processed_tx_hashes: HashSet<String> = HashSet::new();

            while let Some(log) = stream.next().await {
                let transaction_hash = match log.transaction_hash {
                    Some(hash) => format!("{:?}", hash),
                    None => {
                        eprintln!("Skipping Holesky event with no transaction hash at {}", Utc::now());
                        continue;
                    }
                };

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
                            eprintln!("Failed to insert deposit (Tx: {}) into DB: {}", transaction_hash, e);
                        }
                        if let Err(e) = db::update_last_processed_block(&holesky_pool, holesky_chain_id, block_number).await {
                            eprintln!("Failed to update last processed block ({}) for Holesky: {}", block_number, e);
                        }
                        processed_tx_hashes.insert(transaction_hash);
                    }
                    Err(e) => eprintln!("Failed to decode Holesky Deposit event (Tx: {}): {}", transaction_hash, e),
                }
            }
        }
    });

    // Base Sepolia Event Listener
    let _target_chain_handle = tokio::spawn({
        let target_chain_provider = target_chain_provider_clone.clone();
        async move {
            let target_chain_contract = TokenBridge::new(target_chain_bridge_address, target_chain_provider.clone());
            let event = target_chain_contract.deposit_filter();
            let filter = event.filter
                .from_block(target_chain_start_block)
                .topic1(target_chain_test_token);

            let mut stream = loop {
                match target_chain_provider.subscribe_logs(&filter).await {
                    Ok(stream) => {
                        println!("Successfully subscribed to Base Sepolia logs");
                        break stream;
                    }
                    Err(e) => {
                        eprintln!("Failed to subscribe to Base Sepolia logs: {}. Retrying in 60s...", e);
                        sleep(Duration::from_secs(60)).await;
                    }
                }
            };

            let mut processed_tx_hashes: HashSet<String> = HashSet::new();

            while let Some(log) = stream.next().await {
                let transaction_hash = match log.transaction_hash {
                    Some(hash) => format!("{:?}", hash),
                    None => {
                        eprintln!("Skipping Base Sepolia event with no transaction hash at {}", Utc::now());
                        continue;
                    }
                };

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
                            eprintln!("Failed to insert deposit (Tx: {}) into DB: {}", transaction_hash, e);
                        }
                        if let Err(e) = db::update_last_processed_block(&target_chain_pool, target_chain_id, block_number).await {
                            eprintln!("Failed to update last processed block ({}) for Base Sepolia: {}", block_number, e);
                        }
                        processed_tx_hashes.insert(transaction_hash);
                    }
                    Err(e) => eprintln!("Failed to decode Base Sepolia Deposit event (Tx: {}): {}", transaction_hash, e),
                }
            }
        }
    });

    // Finality Confirmation Loop
    let _confirmation_handle = tokio::spawn(async move {
        let confirmation_blocks = 12;

        loop {
            let holesky_block = match holesky_provider.get_block_number().await {
                Ok(block) => block.as_u64() as i64,
                Err(e) => {
                    eprintln!("Failed to get Holesky block number: {}. Retrying in 60s...", e);
                    sleep(Duration::from_secs(60)).await;
                    continue;
                }
            };
            let target_block = match target_chain_provider.get_block_number().await {
                Ok(block) => block.as_u64() as i64,
                Err(e) => {
                    eprintln!("Failed to get Base Sepolia block number: {}. Retrying in 60s...", e);
                    sleep(Duration::from_secs(60)).await;
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
                        "Updated finality for {} deposits on Holesky up to block {}",
                        rows_affected,
                        holesky_block - confirmation_blocks
                    );
                }
                Err(e) => {
                    eprintln!("Failed to update finality for Holesky deposits up to block {}: {}", 
                        holesky_block - confirmation_blocks, e);
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
                        "Updated finality for {} deposits on Base Sepolia up to block {}",
                        rows_affected,
                        target_block - confirmation_blocks
                    );
                }
                Err(e) => {
                    eprintln!("Failed to update finality for Base Sepolia deposits up to block {}: {}", 
                        target_block - confirmation_blocks, e);
                }
            }

            sleep(Duration::from_secs(60)).await;
        }
    });

    // Transaction Processing Loop
    let holesky_client_clone = holesky_client.clone();
    let target_client_clone = target_client.clone();
    let _tx_handle = tokio::spawn(async move {
        let holesky_contract = TokenBridge::new(holesky_bridge_address, holesky_client_clone.clone());
        let target_contract = TokenBridge::new(target_chain_bridge_address, target_client_clone.clone());

        loop {
            println!("Checking for unprocessed deposits at {}", Utc::now());
            let deposits = match db::get_unprocessed_deposits(&tx_pool).await {
                Ok(deposits) => deposits,
                Err(e) => {
                    eprintln!("Failed to fetch unprocessed deposits: {}. Retrying in 60s...", e);
                    sleep(Duration::from_secs(60)).await;
                    continue;
                }
            };

            if deposits.is_empty() {
                println!("No unprocessed deposits found at {}", Utc::now());
                sleep(Duration::from_secs(60)).await;
                continue;
            }

            println!("Found {} unprocessed deposits at {}", deposits.len(), Utc::now());

            for deposit in deposits {
                println!("Processing deposit at {}: {:?}", Utc::now(), deposit);
                let deposit_token = match H160::from_str(&deposit.token_address) {
                    Ok(addr) => addr,
                    Err(e) => {
                        eprintln!("Failed to parse token address for deposit {}: {}", deposit.deposit_id, e);
                        continue;
                    }
                };

                let (contract, token_to_distribute) = if deposit.chain_id == holesky_chain_id {
                    if deposit_token != holesky_test_token {
                        eprintln!("Unexpected token for Holesky deposit {}: {}", deposit.deposit_id, deposit.token_address);
                        continue;
                    }
                    (&target_contract, target_chain_test_token)
                } else if deposit.chain_id == target_chain_id {
                    if deposit_token != target_chain_test_token {
                        eprintln!("Unexpected token for Base Sepolia deposit {}: {}", deposit.deposit_id, deposit.token_address);
                        continue;
                    }
                    (&holesky_contract, holesky_test_token)
                } else {
                    eprintln!("Unknown chain_id for deposit {}: {}", deposit.deposit_id, deposit.chain_id);
                    continue;
                };

                let to_address = match H160::from_str(&deposit.to_address) {
                    Ok(addr) => addr,
                    Err(e) => {
                        eprintln!("Failed to parse to_address for deposit {}: {}", deposit.deposit_id, e);
                        continue;
                    }
                };

                let amount = match U256::from_dec_str(&deposit.amount) {
                    Ok(amount) => amount,
                    Err(e) => {
                        eprintln!("Failed to parse amount '{}' for deposit {}: {}", deposit.amount, deposit.deposit_id, e);
                        continue;
                    }
                };

                let nonce = match U256::from_dec_str(&deposit.nonce) {
                    Ok(nonce) => nonce,
                    Err(e) => {
                        eprintln!("Failed to parse nonce '{}' for deposit {}: {}", deposit.nonce, deposit.deposit_id, e);
                        continue;
                    }
                };

                let is_processed = match contract.processed_deposits(nonce).call().await {
                    Ok(processed) => processed,
                    Err(e) => {
                        eprintln!("Failed to check processedDeposits for deposit {} (nonce {}): {}", 
                            deposit.deposit_id, nonce, e);
                        true // Assume processed to avoid repeated failures
                    }
                };
                if is_processed {
                    println!("Deposit with nonce {} already processed for deposit {} at {}", 
                        nonce, deposit.deposit_id, Utc::now());
                    if let Err(e) = db::update_deposit_status(&tx_pool, deposit.deposit_id, true, true).await {
                        eprintln!("Failed to update deposit status for deposit {}: {}", deposit.deposit_id, e);
                    }
                    continue;
                };

                let distribution_chain = if deposit.chain_id == holesky_chain_id {
                    target_chain_id
                } else {
                    holesky_chain_id
                };

                println!("Distributing for deposit at {}: {:?}", Utc::now(), deposit);
                let call = contract.distribute(token_to_distribute, to_address, amount, nonce);
                let pending_tx = send_with_retry(|| call.send(), 3).await;
                match pending_tx {
                    Ok(tx) => {
                        match tx.await {
                            Ok(Some(receipt)) => {
                                if receipt.status == Some(1.into()) {
                                    println!(
                                        "Distribution successful for deposit {} on {}: tx hash {:?}", 
                                        deposit.deposit_id, distribution_chain, receipt.transaction_hash
                                    );
                                    if let Err(e) = db::update_deposit_status(&tx_pool, deposit.deposit_id, true, true).await {
                                        eprintln!("Failed to update deposit status for deposit {}: {}", deposit.deposit_id, e);
                                    }
                                } else {
                                    eprintln!(
                                        "Distribution tx reverted for deposit {} on {}: tx hash {:?}", 
                                        deposit.deposit_id, distribution_chain, receipt.transaction_hash
                                    );
                                }
                            }
                            Ok(None) => {
                                eprintln!(
                                    "Transaction receipt not found for deposit {} on {}", 
                                    deposit.deposit_id, distribution_chain
                                );
                            }
                            Err(e) => {
                                eprintln!(
                                    "Failed to confirm distribute tx for deposit {} on {}: {}", 
                                    deposit.deposit_id, distribution_chain, e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to send distribute tx for deposit {} on {}: {}", 
                            deposit.deposit_id, distribution_chain, e);
                    }
                }
            }

            sleep(Duration::from_secs(60)).await;
        }
    });

    // Shutdown Handling
    tokio::signal::ctrl_c().await?;
    println!("Received shutdown signal. Pausing contracts...");
    pause_contracts(&holesky_client, &target_client, holesky_bridge_address, target_chain_bridge_address).await;
    println!("Shutting down...");
    Ok(())
}

// Retry Logic for Transactions
async fn send_with_retry<F, Fut, P, T>(f: F, max_retries: usize) -> Result<T, Box<dyn std::error::Error + Send + Sync>>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, ethers::contract::ContractError<SignerMiddleware<P, LocalWallet>>>>,
    P: ethers::providers::Middleware + 'static,
{
    let mut attempts = 0;
    loop {
        match f().await {
            Ok(tx) => return Ok(tx),
            Err(e) if attempts < max_retries => {
                attempts += 1;
                eprintln!("Attempt {} failed: {}. Retrying in 5s...", attempts, e);
                sleep(Duration::from_secs(5)).await;
            }
            Err(e) => return Err(Box::new(e)),
        }
    }
}

// Pause Contracts on Shutdown
async fn pause_contracts(
    holesky_client: &Arc<SignerMiddleware<Arc<Provider<Ws>>, LocalWallet>>,
    target_client: &Arc<SignerMiddleware<Arc<Provider<Ws>>, LocalWallet>>,
    holesky_bridge: H160,
    target_bridge: H160,
) {
    let holesky_contract = TokenBridge::new(holesky_bridge, holesky_client.clone());
    let target_contract = TokenBridge::new(target_bridge, target_client.clone());

    if let Ok(tx) = holesky_contract.pause().send().await {
        println!("Paused Holesky contract: {:?}", tx.tx_hash());
    }
    if let Ok(tx) = target_contract.pause().send().await {
        println!("Paused Base Sepolia contract: {:?}", tx.tx_hash());
    }
}

// Unpause Contracts on Startup
async fn unpause_contracts(
    holesky_client: &Arc<SignerMiddleware<Arc<Provider<Ws>>, LocalWallet>>,
    target_client: &Arc<SignerMiddleware<Arc<Provider<Ws>>, LocalWallet>>,
    holesky_bridge: H160,
    target_bridge: H160,
) {
    let holesky_contract = TokenBridge::new(holesky_bridge, holesky_client.clone());
    let target_contract = TokenBridge::new(target_bridge, target_client.clone());

    // Attempt to unpause Holesky contract
    match holesky_contract.unpause().send().await {
        Ok(tx) => println!("Unpaused Holesky contract: {:?}", tx.tx_hash()),
        Err(e) => eprintln!("Failed to send unpause transaction for Holesky contract: {}", e),
    }

    // Attempt to unpause Base Sepolia contract
    match target_contract.unpause().send().await {
        Ok(tx) => println!("Unpaused Base Sepolia contract: {:?}", tx.tx_hash()),
        Err(e) => eprintln!("Failed to send unpause transaction for Base Sepolia contract: {}", e),
    }
}