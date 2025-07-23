use ethers::{
    prelude::*,
    providers::{Http, Provider},
    signers::{LocalWallet, Signer},
    types::{Address, Bytes, H256, U256},
    utils::hex,
    middleware::SignerMiddleware,
};
use once_cell::sync::Lazy;
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    str::FromStr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Instant,
};
use tokio::time::Duration;
use tracing::{error, info, warn};

// Constants for maximum performance
static TARGET_FROM: Lazy<Address> = Lazy::new(|| {
    Address::from_str("0xc2Db89B2Bd434ceAc6C74FBc0B2ad3a280e66DB0").unwrap()
});
static TARGET_TO: Lazy<Address> = Lazy::new(|| {
    Address::from_str("0x26759dbB201aFbA361Bec78E097Aa3942B0b4AB8").unwrap()
});
static OLD_METHOD_ID: &[u8] = &[0x38, 0x0f, 0x9c, 0x38];
static NEW_METHOD_ID: &[u8] = &[0xb4, 0x20, 0x6d, 0xd2];

const CHECK_RPC_URL: &str = "https://ancient-holy-dinghy.base-mainnet.quiknode.pro/f98091ee17de19d49b9b7962861424b8df12ca09";
const SEND_RPC_URL: &str = "https://mainnet-preconf.base.org";
const POLL_INTERVAL_MS: u64 = 5; // Ultra-aggressive polling - reduced from 10ms
const GAS_LIMIT: u64 = 200_000;
const GAS_MULTIPLIER: u64 = 2; // 2x gas price for testing

struct UltraFastBot {
    check_provider: Provider<Http>,
    signer_middleware: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>,
    processed_txs: Arc<Mutex<HashSet<H256>>>, // Changed to Mutex for speed
    total_blocks_checked: AtomicU64,
    total_txs_sent: AtomicU64,
}

impl UltraFastBot {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        dotenv::dotenv().ok();
        
        let private_key = std::env::var("PRIVATE_KEY")
            .expect("PRIVATE_KEY environment variable not set");
        
        // Create ultra-optimized HTTP clients
        let check_provider = Provider::<Http>::try_from(CHECK_RPC_URL)?;
        let send_provider = Provider::<Http>::try_from(SEND_RPC_URL)?;
        
        let wallet: LocalWallet = private_key.parse::<LocalWallet>()?
            .with_chain_id(8453u64);
        
        // Pre-create signer middleware to avoid creation overhead
        let signer_middleware = Arc::new(SignerMiddleware::new(send_provider, wallet));
        
        Ok(Self {
            check_provider,
            signer_middleware,
            processed_txs: Arc::new(Mutex::new(HashSet::with_capacity(10000))),
            total_blocks_checked: AtomicU64::new(0),
            total_txs_sent: AtomicU64::new(0),
        })
    }

    async fn check_pending_block(&self) -> Result<(), Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        
        // ULTRA-FAST: Direct JSON-RPC call (fastest possible)
        let response: Value = self.check_provider
            .request("eth_getBlockByNumber", json!(["pending", true]))
            .await?;

        // ULTRA-FAST: Process transactions immediately
        if let Some(transactions) = response.get("transactions").and_then(|t| t.as_array()) {
            for tx_value in transactions {
                // ZERO-COPY: Extract values directly without cloning
                if let (Some(hash_str), Some(from_str), Some(to_str), Some(input_str)) = (
                    tx_value.get("hash").and_then(|h| h.as_str()),
                    tx_value.get("from").and_then(|f| f.as_str()),
                    tx_value.get("to").and_then(|t| t.as_str()),
                    tx_value.get("input").and_then(|i| i.as_str()),
                ) {
                    // ULTRA-FAST: Parse addresses only if we need them
                    if from_str.eq_ignore_ascii_case("0xc2Db89B2Bd434ceAc6C74FBc0B2ad3a280e66DB0") &&
                       to_str.eq_ignore_ascii_case("0x26759dbB201aFbA361Bec78E097Aa3942B0b4AB8") &&
                       input_str.len() >= 10 &&
                       input_str[2..10].eq_ignore_ascii_case("380f9c38") {
                        
                        let tx_hash = H256::from_str(hash_str)?;
                        
                        // ULTRA-FAST: Quick duplicate check with Mutex (faster than RwLock)
                        {
                            let mut processed = self.processed_txs.lock().unwrap();
                            if processed.contains(&tx_hash) {
                                continue;
                            }
                            processed.insert(tx_hash);
                        }

                        let found_time = start_time.elapsed();
                        info!("🎯 Target found: {} | Find: {:.2}ms", hash_str, found_time.as_secs_f64() * 1000.0);

                        // ZERO-DELAY: Submit immediately without spawning
                        let submission_start = Instant::now();
                        if let Err(e) = self.submit_frontrun_immediately(tx_value, submission_start).await {
                            error!("Submission failed: {}", e);
                        }

                        // EARLY EXIT: Stop processing other transactions (save time)
                        break;
                    }
                }
            }
        }

        let total_time = start_time.elapsed();
        let blocks_checked = self.total_blocks_checked.fetch_add(1, Ordering::Relaxed) + 1;
        
        if blocks_checked % 1000 == 0 {
            info!("📊 Blocks: {} | Time: {:.2}ms | Sent: {}", 
                  blocks_checked, 
                  total_time.as_secs_f64() * 1000.0,
                  self.total_txs_sent.load(Ordering::Relaxed));
        }

        Ok(())
    }

    async fn submit_frontrun_immediately(
        &self,
        tx_value: &Value,
        submission_start: Instant,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // ULTRA-FAST: Extract required values directly
        let value_str = tx_value.get("value").and_then(|v| v.as_str()).unwrap_or("0x0");
        let gas_price_str = tx_value.get("gasPrice").and_then(|g| g.as_str()).unwrap_or("0x0");
        let input_str = tx_value.get("input").and_then(|i| i.as_str()).unwrap_or("");

        // ZERO-COPY: Process input data efficiently
        if input_str.len() < 10 {
            return Err("Input too short".into());
        }

        // ULTRA-FAST: Build modified input directly
        let mut modified_input = String::with_capacity(input_str.len());
        modified_input.push_str("0xb4206dd2"); // NEW_METHOD_ID
        modified_input.push_str(&input_str[10..]);

        // ULTRA-FAST: Parse values
        let value = U256::from_str(value_str)?;
        let original_gas_price = U256::from_str(gas_price_str)?;
        let boosted_gas_price = original_gas_price * U256::from(GAS_MULTIPLIER);

        // ULTRA-FAST: Create transaction with minimal overhead
        let tx = TransactionRequest::new()
            .to(*TARGET_TO)
            .value(value)
            .data(Bytes::from(hex::decode(&modified_input[2..])?))
            .gas(U256::from(GAS_LIMIT))
            .gas_price(boosted_gas_price);

        // ZERO-DELAY: Submit immediately (no async spawning)
        let pending_tx = self.signer_middleware
            .send_transaction(tx, None)
            .await?;

        let submission_time = submission_start.elapsed();
        let tx_count = self.total_txs_sent.fetch_add(1, Ordering::Relaxed) + 1;
        
        info!("⚡ TX submitted: {} | Submit: {:.2}ms | Gas: {}x | Total: {}", 
              pending_tx.tx_hash(), 
              submission_time.as_secs_f64() * 1000.0,
              GAS_MULTIPLIER,
              tx_count);

        Ok(())
    }

    async fn start_monitoring(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("🚀 Starting ULTRA-FAST MEV monitoring...");
        info!("⚡ Polling: {}ms | Gas: {}x", POLL_INTERVAL_MS, GAS_MULTIPLIER);
        info!("🎯 Target: {} -> {}", *TARGET_FROM, *TARGET_TO);
        
        let mut interval = tokio::time::interval(Duration::from_millis(POLL_INTERVAL_MS));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;
            
            if let Err(e) = self.check_pending_block().await {
                warn!("Block check error: {}", e);
            }
        }
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Ultra-minimal logging for maximum performance
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .compact()
        .init();

    let available_parallelism = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    
    info!("🔥 Ultra-Fast MEV Bot | Cores: {}", available_parallelism);

    // Create and start the ultra-optimized bot
    let bot = UltraFastBot::new().await?;
    
    // Start monitoring with zero-delay
    bot.start_monitoring().await?;

    Ok(())
}
