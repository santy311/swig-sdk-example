//! MultiWalletManager Example
//!
//! This example demonstrates batch operations across multiple Swig wallets using
//! the MultiWalletManager. Essential for:
//! - Airdrop distributions
//! - Mass wallet migrations
//! - Batch payments
//! - Portfolio rebalancing
//!
//! Run with: `cargo run --example multi_wallet_manager`

use rand::Rng;
use solana_client::rpc_client::RpcClient;
use solana_keypair::Keypair;
use solana_sdk::{
    instruction::Instruction, pubkey::Pubkey, system_instruction, transaction::Transaction,
};
use solana_signer::{EncodableKey, Signer};
use std::path::Path;
use swig_sdk::{BatchConfig, BatchStrategy, Ed25519ClientRole, MultiWalletManager, SwigWallet};

const RPC_URL: &str = "https://api.devnet.solana.com";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let authority = load_or_create_keypair("authority.json");
    println!("Authority: {}", authority.pubkey());

    let rpc_client = RpcClient::new(RPC_URL);

    // =========================================================================
    // 1. CREATE MULTIPLE WALLETS
    // =========================================================================
    println!("\n=== Creating Multiple Wallets ===");

    let mut wallet_ids: Vec<([u8; 32], u32)> = Vec::new();
    let mut wallet_addresses: Vec<Pubkey> = Vec::new();

    for i in 0..3 {
        let swig_id: [u8; 32] = rand::thread_rng().r#gen();

        let wallet = SwigWallet::builder()
            .with_swig_id(swig_id)
            .with_client_role(Box::new(Ed25519ClientRole::new(authority.pubkey())))
            .with_rpc_url(RPC_URL.to_string())
            .with_fee_payer(&authority)
            .with_authority_keypair(Some(&authority))
            .create()?;

        let info = wallet.get_info()?;
        wallet_addresses.push(info.swig_wallet_address);

        println!("Created wallet {}: {}", i + 1, hex::encode(swig_id));
        wallet_ids.push((swig_id, 0)); // (swig_id, role_id)
    }

    // =========================================================================
    // 2. FUND ALL WALLETS
    // =========================================================================
    println!("\n=== Funding All Wallets ===");

    let fund_amount = 10_000_000; // 0.01 SOL each
    fund_wallet(&rpc_client, &authority, &wallet_addresses, fund_amount)?;
    println!("Funded all wallets");

    // =========================================================================
    // 3. CREATE MULTI-WALLET MANAGER
    // =========================================================================
    println!("\n=== Creating MultiWalletManager ===");

    let rpc_client = RpcClient::new(RPC_URL);
    let client_role = Box::new(Ed25519ClientRole::new(authority.pubkey()));

    let mut manager = MultiWalletManager::new(
        client_role,
        &authority,       // fee payer
        Some(&authority), // authority keypair (for Ed25519)
        rpc_client,
    );

    println!("Manager created for {} wallets", wallet_ids.len());

    // =========================================================================
    // 4. BATCH CONFIGURATION
    // =========================================================================
    println!("\n=== Batch Configuration Options ===");

    // Default configuration
    let _default_config = BatchConfig::default();
    println!("Default config created");

    // Optimized for high throughput
    let _fast_config = BatchConfig::default()
        .with_strategy(BatchStrategy::BinarySearchFailures)
        .with_max_accounts(64)
        .with_max_tx_size(1024)
        .with_max_retries(3)
        .with_retry_delay(500)
        .with_num_threads(4); // Parallel execution

    println!("Fast config: parallel execution with 4 threads");

    // Conservative for reliability
    let _safe_config = BatchConfig::default()
        .with_strategy(BatchStrategy::Simple)
        .with_max_accounts(32)
        .with_max_retries(5)
        .with_retry_delay(1000)
        .with_num_threads(1); // Sequential

    println!("Safe config: sequential execution with more retries");

    // =========================================================================
    // 5. CREATE SOL TRANSFER INSTRUCTIONS
    // =========================================================================
    println!("\n=== Creating SOL Transfer Instructions ===");

    let recipient = Pubkey::new_unique();
    let amount_per_wallet = 1000; // lamports

    let sol_instructions = manager.create_sol_transfer_instructions(
        wallet_ids.clone(),
        recipient,
        amount_per_wallet,
        None, // current_slot (auto-fetched for non-Ed25519)
    )?;

    println!("Created {} instruction batches", sol_instructions.len());

    // =========================================================================
    // 6. CREATE TOKEN TRANSFER INSTRUCTIONS
    // =========================================================================
    println!("\n=== Creating Token Transfer Instructions ===");

    let token_mint = Pubkey::new_unique(); // Replace with real mint
    let token_recipient = Pubkey::new_unique();
    let token_amount = 1000;

    let token_instructions = manager.create_token_transfer_instructions(
        wallet_ids.clone(),
        token_mint,
        token_recipient,
        token_amount,
        None,
    )?;

    println!(
        "Created {} token instruction batches",
        token_instructions.len()
    );

    // =========================================================================
    // 7. CREATE CUSTOM INSTRUCTIONS
    // =========================================================================
    println!("\n=== Creating Custom Instructions ===");

    let custom_instructions = manager.create_instructions(
        wallet_ids.clone(),
        |swig_id, _role_id, swig_wallet_address| {
            // Custom logic per wallet
            // Here we create a unique recipient per wallet
            let recipient = Pubkey::new_unique();
            let amount = 500 + (swig_id[0] as u64 * 10); // Variable amount

            Ok(system_instruction::transfer(
                &swig_wallet_address,
                &recipient,
                amount,
            ))
        },
        None,
    )?;

    println!(
        "Created {} custom instruction batches",
        custom_instructions.len()
    );

    // =========================================================================
    // 8. EXECUTE BATCH OPERATIONS
    // =========================================================================
    println!("\n=== Executing Batch Operations ===");

    let recipient = Pubkey::new_unique();
    let config = BatchConfig::default().with_num_threads(2);

    let result = manager
        .execute_batch(
            wallet_ids.clone(),
            |_swig_id, _role_id, swig_wallet_address| {
                Ok(system_instruction::transfer(
                    &swig_wallet_address,
                    &recipient,
                    1,
                ))
            },
            config,
        )
        .await?;

    // =========================================================================
    // 9. HANDLE BATCH RESULTS
    // =========================================================================
    println!("\n=== Batch Results ===");

    println!("Success: {}", result.is_success());
    println!("Successful operations: {}", result.successful_count());
    println!("Failed operations: {}", result.failed_count());

    // Get successful wallet IDs
    let successful_ids = result.successful_swig_ids();
    println!("\nSuccessful wallets:");
    for id in &successful_ids {
        println!("  {}", hex::encode(id));
    }

    // Get failed wallet IDs
    let failed_ids = result.failed_swig_ids();
    if !failed_ids.is_empty() {
        println!("\nFailed wallets:");
        for id in &failed_ids {
            println!("  {}", hex::encode(id));
        }
    }

    // Detailed success info
    println!("\nSuccessful batches:");
    for batch in &result.successful {
        println!("  Signature: {}", batch.signature);
        println!("  Wallet count: {}", batch.swig_ids.len());
    }

    // Detailed failure info
    if !result.failed.is_empty() {
        println!("\nFailed operations:");
        for failed in &result.failed {
            println!("  Wallet: {}", hex::encode(failed.swig_id));
            println!("  Error: {:?}", failed.error);
        }
    }

    // =========================================================================
    // 10. RETRY FAILED OPERATIONS
    // =========================================================================
    if !failed_ids.is_empty() {
        println!("\n=== Retrying Failed Operations ===");

        let retry_wallet_ids: Vec<_> = failed_ids.iter().map(|id| (*id, 0u32)).collect();

        let retry_config = BatchConfig::default()
            .with_max_retries(5)
            .with_retry_delay(2000);

        let retry_result = manager
            .execute_batch(
                retry_wallet_ids,
                |_swig_id, _role_id, swig_wallet_address| {
                    Ok(system_instruction::transfer(
                        &swig_wallet_address,
                        &recipient,
                        1000,
                    ))
                },
                retry_config,
            )
            .await?;

        println!("Retry success: {}", retry_result.is_success());
    }

    // =========================================================================
    // 11. BATCH STRATEGIES
    // =========================================================================
    println!("\n=== Batch Strategies ===");

    println!("BatchStrategy::Simple (default):");
    println!("  - Sends batches as-is");
    println!("  - If batch fails, all wallets in batch marked failed");
    println!("  - Fast but less granular");

    println!("\nBatchStrategy::BinarySearchFailures:");
    println!("  - When batch fails, recursively splits to find exact failures");
    println!("  - More retries but precise failure detection");
    println!("  - Better for production");

    println!("\n=== Done ===");
    Ok(())
}

/// Fund a swig wallet by transferring SOL from the fee payer.
fn fund_wallet(
    rpc_client: &RpcClient,
    fee_payer: &Keypair,
    wallet_addresses: &[Pubkey],
    amount: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let transfer_ixes = wallet_addresses
        .iter()
        .map(|wallet_address| {
            system_instruction::transfer(&fee_payer.pubkey(), wallet_address, amount)
        })
        .collect::<Vec<Instruction>>();

    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let tx = Transaction::new_signed_with_payer(
        &transfer_ixes,
        Some(&fee_payer.pubkey()),
        &[fee_payer],
        recent_blockhash,
    );
    let signature = rpc_client.send_and_confirm_transaction(&tx)?;
    println!("Funding tx: {}", signature);
    Ok(())
}

fn load_or_create_keypair(path: &str) -> Keypair {
    let path = Path::new(path);
    if path.exists() {
        Keypair::read_from_file(path).expect("Failed to read keypair")
    } else {
        let keypair = Keypair::new();
        keypair
            .write_to_file(path)
            .expect("Failed to write keypair");
        println!("Created new keypair at: {}", path.display());
        keypair
    }
}
