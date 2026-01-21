//! Ed25519 Wallet Example
//!
//! This example demonstrates how to create, load, and sign with a Swig wallet
//! using an Ed25519 (native Solana) authority.
//!
//! Run with: `cargo run --example ed25519_wallet`

use rand::Rng;
use solana_client::rpc_client::RpcClient;
use solana_keypair::Keypair;
use solana_sdk::{pubkey::Pubkey, system_instruction, transaction::Transaction};
use solana_signer::{EncodableKey, Signer};
use std::path::Path;
use swig_sdk::{Ed25519ClientRole, Permission, SwigWallet, authority::AuthorityType};

const RPC_URL: &str = "https://api.devnet.solana.com";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // =========================================================================
    // Setup: Load or create authority keypair
    // =========================================================================
    let authority =
        Keypair::read_from_file(Path::new("authority.json")).expect("Failed to read keypair");
    println!("Authority: {}", authority.pubkey());

    let rpc_client = RpcClient::new(RPC_URL);

    // =========================================================================
    // 1. CREATE A NEW WALLET
    // =========================================================================
    println!("\n=== Creating New Wallet ===");

    // Generate a random swig_id (32 bytes)
    let swig_id: [u8; 32] = rand::thread_rng().r#gen();
    println!("Swig ID: {}", hex::encode(swig_id));

    // Create Ed25519 client role from authority's public key
    let client_role = Ed25519ClientRole::new(authority.pubkey());

    // Build and create the wallet
    let wallet = SwigWallet::builder()
        .with_swig_id(swig_id)
        .with_client_role(Box::new(client_role))
        .with_rpc_url(RPC_URL.to_string())
        .with_fee_payer(&authority)
        .with_authority_keypair(Some(&authority))
        .create()?;

    println!("Wallet created successfully!");
    wallet.display_swig()?;

    // =========================================================================
    // 2. LOAD AN EXISTING WALLET
    // =========================================================================
    println!("\n=== Loading Existing Wallet ===");

    // To load, create a new client role and use .load() instead of .create()
    let client_role = Ed25519ClientRole::new(authority.pubkey());

    let mut loaded_wallet = SwigWallet::builder()
        .with_swig_id(swig_id)
        .with_client_role(Box::new(client_role))
        .with_rpc_url(RPC_URL.to_string())
        .with_fee_payer(&authority)
        .with_authority_keypair(Some(&authority))
        .load()?;

    println!("Wallet loaded successfully!");

    // =========================================================================
    // 3. GET WALLET INFO
    // =========================================================================
    println!("\n=== Wallet Info ===");

    let info = loaded_wallet.get_info()?;
    println!("Config Address: {}", info.swig_config_address);
    println!("Wallet Address: {}", info.swig_wallet_address);
    println!("Balance: {} lamports", info.wallet_balance);
    println!("Number of Roles: {}", info.roles_count);

    // =========================================================================
    // 4. FUND THE WALLET (transfer SOL to the swig wallet)
    // =========================================================================
    println!("\n=== Funding Wallet ===");

    let fund_amount = 10_000_000; // 0.01 SOL
    fund_wallet(
        &rpc_client,
        &authority,
        &info.swig_wallet_address,
        fund_amount,
    )?;
    println!("Funded wallet with {} lamports", fund_amount);

    // =========================================================================
    // 5. ADD A NEW AUTHORITY
    // =========================================================================
    println!("\n=== Adding New Authority ===");

    let new_authority = Keypair::new();
    println!("New authority pubkey: {}", new_authority.pubkey());

    // Add with limited SOL transfer permission (1 SOL)
    let permissions = vec![
        Permission::Sol {
            amount: 1_000_000_000,
            recurring: None,
        },
        Permission::ProgramCurated,
    ];

    let sig = loaded_wallet.add_authority(
        AuthorityType::Ed25519,
        new_authority.pubkey().as_ref(),
        permissions,
    )?;
    println!("Authority added! Signature: {}", sig);

    // =========================================================================
    // 6. SWITCH TO NEW AUTHORITY
    // =========================================================================
    println!("\n=== Switching to New Authority ===");

    // Create a new client role with the new authority's public key
    let new_client_role = Ed25519ClientRole::new(new_authority.pubkey());

    // Reload the wallet with the new authority
    loaded_wallet.switch_authority(1, Box::new(new_client_role), Some(&new_authority))?;

    println!("Switched to new authority!");

    // =========================================================================
    // 7. SIGN A TRANSACTION (as new authority)
    // =========================================================================
    println!("\n=== Signing SOL Transfer (as new authority) ===");

    let recipient = Pubkey::new_unique();
    let amount = 1000; // lamports

    let transfer_ix = system_instruction::transfer(&info.swig_wallet_address, &recipient, amount);

    // Sign and submit the transaction through the swig wallet using the new authority
    let signature = loaded_wallet.sign_v2(vec![transfer_ix], None)?;
    println!("Transfer signed! Signature: {}", signature);

    // =========================================================================
    // 8. CHECK IF WALLET EXISTS
    // =========================================================================
    println!("\n=== Check Wallet Existence ===");

    let exists = SwigWallet::exists(swig_id, RPC_URL)?;
    println!("Wallet exists: {}", exists);

    let fake_id: [u8; 32] = [0; 32];
    let fake_exists = SwigWallet::exists(fake_id, RPC_URL)?;
    println!("Fake wallet exists: {}", fake_exists);

    println!("\n=== Done ===");
    Ok(())
}

/// Fund a swig wallet by transferring SOL from the fee payer.
fn fund_wallet(
    rpc_client: &RpcClient,
    fee_payer: &Keypair,
    wallet_address: &Pubkey,
    amount: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let transfer_ix = system_instruction::transfer(&fee_payer.pubkey(), wallet_address, amount);

    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let tx = Transaction::new_signed_with_payer(
        &[transfer_ix],
        Some(&fee_payer.pubkey()),
        &[fee_payer],
        recent_blockhash,
    );

    let signature = rpc_client.send_and_confirm_transaction(&tx)?;
    println!("Funding tx: {}", signature);

    Ok(())
}
