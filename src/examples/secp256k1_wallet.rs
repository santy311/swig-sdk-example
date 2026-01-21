//! Secp256k1 Wallet Example
//!
//! This example demonstrates how to create, load, and sign with a Swig wallet
//! using a Secp256k1 (Ethereum-compatible) authority.
//!
//! Run with: `cargo run --example secp256k1_wallet`

use alloy_primitives::B256;
use alloy_signer::SignerSync;
use alloy_signer_local::PrivateKeySigner;
use solana_client::rpc_client::RpcClient;
use solana_keypair::Keypair;
use solana_sdk::{pubkey::Pubkey, system_instruction, transaction::Transaction};
use solana_signer::{EncodableKey, Signer};
use std::path::Path;
use swig_sdk::{Permission, Secp256k1ClientRole, SwigWallet, authority::AuthorityType};

const RPC_URL: &str = "https://api.devnet.solana.com";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // =========================================================================
    // Setup: Generate secp256k1 keypair using alloy (Ethereum-compatible)
    // =========================================================================
    // In production, you would use a proper Ethereum wallet or hardware wallet.
    // For this example, we generate a random keypair.

    let (secp_wallet, public_key) = create_secp256k1_wallet();

    println!("Secp256k1 Public Key: {}", hex::encode(&public_key));

    // Fee payer (still needs a Solana keypair for transaction fees)
    let fee_payer =
        Keypair::read_from_file(Path::new("authority.json")).expect("Failed to read keypair");

    println!("Fee Payer: {}", fee_payer.pubkey());

    let rpc_client = RpcClient::new(RPC_URL);

    // =========================================================================
    // 1. CREATE SECP256K1 CLIENT ROLE
    // =========================================================================
    println!("\n=== Creating Secp256k1 Client Role ===");

    // Clone the wallet for use in the signing closure
    let secp_wallet_clone = secp_wallet.clone();
    let client_role = create_secp256k1_client_role(public_key.clone(), secp_wallet_clone);

    // =========================================================================
    // 2. CREATE A NEW WALLET
    // =========================================================================
    println!("\n=== Creating New Wallet ===");

    let swig_id: [u8; 32] = rand::random();
    println!("Swig ID: {}", hex::encode(swig_id));

    let wallet = SwigWallet::builder()
        .with_swig_id(swig_id)
        .with_client_role(Box::new(client_role))
        .with_rpc_url(RPC_URL.to_string())
        .with_fee_payer(&fee_payer)
        // Note: No authority_keypair needed for secp256k1
        // The signing function handles signing internally
        .create()?;

    println!("Wallet created successfully!");
    wallet.display_swig()?;

    // =========================================================================
    // 3. LOAD AN EXISTING WALLET
    // =========================================================================
    println!("\n=== Loading Existing Wallet ===");

    // Create a fresh client role for loading
    let secp_wallet_clone = secp_wallet.clone();
    let client_role = create_secp256k1_client_role(public_key.clone(), secp_wallet_clone);

    let mut loaded_wallet = SwigWallet::builder()
        .with_swig_id(swig_id)
        .with_client_role(Box::new(client_role))
        .with_rpc_url(RPC_URL.to_string())
        .with_fee_payer(&fee_payer)
        .load()?;

    println!("Wallet loaded successfully!");

    // =========================================================================
    // 4. GET WALLET INFO
    // =========================================================================
    println!("\n=== Wallet Info ===");

    let info = loaded_wallet.get_info()?;
    println!("Config Address: {}", info.swig_config_address);
    println!("Wallet Address: {}", info.swig_wallet_address);
    println!("Balance: {} lamports", info.wallet_balance);

    // =========================================================================
    // 5. FUND THE WALLET
    // =========================================================================
    println!("\n=== Funding Wallet ===");

    let fund_amount = 10_000_000; // 0.01 SOL
    fund_wallet(
        &rpc_client,
        &fee_payer,
        &info.swig_wallet_address,
        fund_amount,
    )?;
    println!("Funded wallet with {} lamports", fund_amount);

    // =========================================================================
    // 6. ADD SECP256K1 AUTHORITY TO EXISTING WALLET
    // =========================================================================
    println!("\n=== Adding Secp256k1 Authority ===");

    // Generate another secp256k1 keypair
    let (new_secp_wallet, new_public_key) = create_secp256k1_wallet();

    // Add 0x04 prefix for uncompressed public key format (65 bytes total)
    let mut new_public_key_with_prefix = vec![0x04];
    new_public_key_with_prefix.extend_from_slice(&new_public_key);
    println!("New authority: {}", hex::encode(&new_public_key_with_prefix));

    let permissions = vec![
        Permission::Sol {
            amount: 500_000_000, // 0.5 SOL limit
            recurring: None,
        },
        Permission::ProgramCurated,
    ];

    let sig =
        loaded_wallet.add_authority(AuthorityType::Secp256k1, &new_public_key_with_prefix, permissions)?;
    println!("Authority added! Signature: {}", sig);

    // =========================================================================
    // 7. SWITCH TO NEW AUTHORITY
    // =========================================================================
    println!("\n=== Switching to New Authority ===");

    // Create a new client role with the new authority's credentials
    let new_client_role = create_secp256k1_client_role(new_public_key, new_secp_wallet);

    // Switch to the new authority (role index 1)
    loaded_wallet.switch_authority(1, Box::new(new_client_role), None)?;

    println!("Switched to new authority!");

    // =========================================================================
    // 8. SIGN A TRANSACTION (as new authority)
    // =========================================================================
    println!("\n=== Signing SOL Transfer (as new authority) ===");

    let recipient = Pubkey::new_unique();
    let amount = 1000;

    let transfer_ix = system_instruction::transfer(&info.swig_wallet_address, &recipient, amount);

    // Secp256k1 signing automatically handles:
    // - Fetching current slot for replay protection
    // - Including slot in signed message
    // - Incrementing odometer
    let signature = loaded_wallet.sign_v2(vec![transfer_ix], None)?;
    println!("Transfer signed! Signature: {}", signature);

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

/// Create a secp256k1 keypair using alloy (Ethereum-compatible).
/// Returns the private key signer and 64-byte uncompressed public key (without 0x04 prefix).
fn create_secp256k1_wallet() -> (PrivateKeySigner, Vec<u8>) {
    let wallet = PrivateKeySigner::random();
    let secp_pubkey = wallet
        .credential()
        .verifying_key()
        .to_encoded_point(false) // false = uncompressed format
        .to_bytes();
    // Strip the 0x04 prefix, keep the 64-byte public key
    (wallet, secp_pubkey.as_ref()[1..].to_vec())
}

/// Create a Secp256k1ClientRole with the signing function.
/// Uses alloy_signer_local for Ethereum-compatible signing.
fn create_secp256k1_client_role(
    public_key: Vec<u8>,
    secp_wallet: PrivateKeySigner,
) -> Secp256k1ClientRole {
    // The signing function takes a payload and returns a 65-byte recoverable signature
    // IMPORTANT: Only use the first 32 bytes of the payload as the message hash
    let signing_fn = Box::new(move |payload: &[u8]| -> [u8; 65] {
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&payload[..32]);
        let hash = B256::from(hash);
        secp_wallet.sign_hash_sync(&hash).unwrap().as_bytes()
    });

    Secp256k1ClientRole::new(public_key.into_boxed_slice(), signing_fn)
}
