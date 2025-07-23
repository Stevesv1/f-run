use ethers::{
    prelude::*,
    providers::{Http, Provider},
    signers::{LocalWallet, Signer},
    types::{Address, BlockNumber, Bytes, H256, U256},
    utils::hex,
    middleware::SignerMiddleware,
};
use once_cell::sync::Lazy;
use std::{
    collections::HashSet,
    str::FromStr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Instant,
};
use tokio::{sync::RwLock, time::Duration};
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
const POLL_INTERVAL_MS: u64 = 10; // Extremely aggressive polling
const GAS_LIMIT: u64 = 200_000;
const GAS_MULTIPLIER: u64 = 1; // 15x gas price

type SharedProcessedTxs = Arc<RwLock<HashSet<H256>>>;

struct OptimizedBot {
    check_provider: Provider<Http>,
    send_provider: Provider<Http>,
    wallet: LocalWallet,
    processed_txs: SharedProcessedTxs,
    total_blocks_checked: AtomicU64,
    total_txs_sent: Arc<AtomicU64>,
}

impl OptimizedBot {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        dotenv::dotenv().ok();
        
        let private_key = std::env::var("PRIVATE_KEY")
            .expect("PRIVATE_KEY environment variable not set");
        
        // Create optimized HTTP clients with connection pooling
        let check_provider = Provider::<Http>::try_from(CHECK_RPC_URL)?
            .interval(Duration::from_millis(POLL_INTERVAL_MS));
        
        let send_provider = Provider::<Http>::try_from(SEND_RPC_URL)?;
        
        let wallet: LocalWallet = private_key.parse::<LocalWallet>()?
            .with_chain_id(8453u64); // Base mainnet chain ID
        
        Ok(Self {
            check_provider,
            send_provider,
            wallet,
            processed_txs: Arc::new(RwLock::new(HashSet::with_capacity(10000))),
            total_blocks_checked: AtomicU64::new(0),
            total_txs_sent: Arc::new(AtomicU64::new(0)),
        })
    }

    async fn check_pending_block(&self) -> Result<(), Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        
        // Use ethers-rs native method for getting pending block with transactions
        let pending_block = match self.check_provider.get_block_with_txs(BlockNumber::Pending).await {
            Ok(Some(block)) => block,
            Ok(None) => {
                // No pending block available
                return Ok(());
            }
            Err(e) => {
                // Log error and continue
                warn!("Failed to get pending block: {}", e);
                return Ok(());
            }
        };
        
        for tx in &pending_block.transactions {
            let tx_hash = tx.hash;
            
            // Quick check if already processed
            {
                let processed_read = self.processed_txs.read().await;
                if processed_read.contains(&tx_hash) {
                    continue;
                }
            }

            // Check if this is our target transaction
            let from_addr = tx.from;
            if let Some(to_addr) = tx.to {
                    if from_addr == *TARGET_FROM && 
                       to_addr == *TARGET_TO && 
                       tx.input.len() >= 4 &&
                       &tx.input[0..4] == OLD_METHOD_ID {
                        
                        let found_time = start_time.elapsed();
                        info!("🎯 Target transaction found: {:?} | Find time: {:.2}ms", 
                              tx_hash, found_time.as_secs_f64() * 1000.0);

                        // Mark as processed immediately
                        {
                            let mut processed_write = self.processed_txs.write().await;
                            processed_write.insert(tx_hash);
                        }

                        // Spawn immediate transaction submission
                        let bot_clone = self.clone_for_submission();
                        let tx_data = TxData {
                            to: to_addr,
                            value: tx.value,
                            gas_price: tx.gas_price.unwrap_or(U256::zero()),
                            input: hex::encode(&tx.input),
                        };
                        let detection_instant = Instant::now();
                        
                        tokio::spawn(async move {
                            if let Err(e) = bot_clone.submit_frontrun_tx(tx_data, detection_instant).await {
                                error!("Failed to submit frontrun transaction: {}", e);
                            }
                        });
                    }
                }
        }

        let total_time = start_time.elapsed();
        let blocks_checked = self.total_blocks_checked.fetch_add(1, Ordering::Relaxed) + 1;
        
        if blocks_checked % 1000 == 0 {
            info!("📊 Blocks checked: {} | Last block time: {:.2}ms | Txs sent: {}", 
                  blocks_checked, 
                  total_time.as_secs_f64() * 1000.0,
                  self.total_txs_sent.load(Ordering::Relaxed));
        }

        Ok(())
    }

    fn clone_for_submission(&self) -> SubmissionBot {
        let signer_middleware = Arc::new(SignerMiddleware::new(
            self.send_provider.clone(),
            self.wallet.clone(),
        ));
        SubmissionBot {
            wallet_with_provider: signer_middleware,
            total_txs_sent: Arc::clone(&self.total_txs_sent),
        }
    }



    async fn start_monitoring(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("🚀 Starting ultra-high-performance MEV monitoring...");
        info!("⚡ Polling interval: {}ms", POLL_INTERVAL_MS);
        info!("🎯 Target: {} -> {}", *TARGET_FROM, *TARGET_TO);
        
        let mut interval = tokio::time::interval(Duration::from_millis(POLL_INTERVAL_MS));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;
            
            if let Err(e) = self.check_pending_block().await {
                warn!("Error checking pending block: {}", e);
            }
        }
    }
}

#[derive(Clone)]
struct SubmissionBot {
    wallet_with_provider: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>,
    total_txs_sent: Arc<AtomicU64>,
}

#[derive(Debug)]
struct TxData {
    to: Address,
    value: U256,
    gas_price: U256,
    input: String,
}

impl SubmissionBot {
    async fn submit_frontrun_tx(
        &self, 
        tx_data: TxData, 
        detection_instant: Instant
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Create modified transaction data with new method ID
        let input_bytes = hex::decode(&tx_data.input).map_err(|_| "Invalid hex input")?;
        if input_bytes.len() < 4 {
            return Err("Input too short".into());
        }

        // Replace the method ID (first 4 bytes) with the new one
        let mut modified_input = NEW_METHOD_ID.to_vec();
        modified_input.extend_from_slice(&input_bytes[4..]);

        // Calculate boosted gas price
        let boosted_gas_price = tx_data.gas_price * U256::from(GAS_MULTIPLIER);

        // Create transaction
        let tx = TransactionRequest::new()
            .to(tx_data.to)
            .value(tx_data.value)
            .data(Bytes::from(modified_input))
            .gas(U256::from(GAS_LIMIT))
            .gas_price(boosted_gas_price);

        // Send transaction immediately using signer middleware
        let pending_tx = self.wallet_with_provider
            .send_transaction(tx, None)
            .await?;

        let submission_time = detection_instant.elapsed();
        let tx_count = self.total_txs_sent.fetch_add(1, Ordering::Relaxed) + 1;
        
        info!("⚡ Frontrun TX submitted: {} | Submission time: {:.2}ms | Total sent: {}", 
              pending_tx.tx_hash(), 
              submission_time.as_secs_f64() * 1000.0,
              tx_count);

        Ok(())
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize high-performance tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Set tokio runtime to use all available cores
    let available_parallelism = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    
    info!("🔥 Initializing MEV bot with {} cores", available_parallelism);

    // Create and start the optimized bot
    let bot = OptimizedBot::new().await?;
    
    // Start monitoring with maximum performance
    bot.start_monitoring().await?;

    Ok(())
}
