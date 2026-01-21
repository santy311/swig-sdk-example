//! Secp256r1 (P-256) Wallet Example
//!
//! This example demonstrates how to create, load, and sign with a Swig wallet
//! using a Secp256r1 (WebAuthn/Passkey compatible) authority.
//!
//! Run with: `cargo run --example secp256r1_wallet`

use openssl::{
    bn::BigNumContext,
    ec::{EcGroup, EcKey, PointConversionForm},
    nid::Nid,
    pkey::Private,
};
use solana_client::rpc_client::RpcClient;
use solana_keypair::Keypair;
use solana_sdk::{pubkey::Pubkey, system_instruction, transaction::Transaction};
use solana_signer::{EncodableKey, Signer as SolanaSigner};
use std::path::Path;
use swig_sdk::{
    Permission, SwigWallet, authority::AuthorityType, client_role::Secp256r1ClientRole,
};

const RPC_URL: &str = "https://api.devnet.solana.com";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // =========================================================================
    // Setup: Generate secp256r1 (P-256) keypair using OpenSSL
    // =========================================================================
    // In production, this would come from WebAuthn/Passkey.
    // For this example, we generate a key using OpenSSL.

    let (signing_key, public_key) = create_secp256r1_keypair();

    println!(
        "Secp256r1 Public Key (compressed): {}",
        hex::encode(&public_key)
    );

    // Fee payer (still needs a Solana keypair for transaction fees)
    let fee_payer =
        Keypair::read_from_file(Path::new("authority.json")).expect("Failed to read keypair");
    println!("Fee Payer: {}", fee_payer.pubkey());

    let rpc_client = RpcClient::new(RPC_URL);

    // =========================================================================
    // 1. CREATE SECP256R1 CLIENT ROLE
    // =========================================================================
    println!("\n=== Creating Secp256r1 Client Role ===");

    // Clone the signing key for use in the client role
    let signing_key_der = signing_key.private_key_to_der()?;
    let client_role = create_secp256r1_client_role(public_key, signing_key_der.clone())?;

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
        .create()?;

    println!("Wallet created successfully!");
    wallet.display_swig()?;

    // =========================================================================
    // 3. LOAD AN EXISTING WALLET
    // =========================================================================
    println!("\n=== Loading Existing Wallet ===");

    let client_role = create_secp256r1_client_role(public_key, signing_key_der.clone())?;

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
    // 6. SIGN A TRANSACTION
    // =========================================================================
    println!("\n=== Signing SOL Transfer ===");

    let recipient = Pubkey::new_unique();
    let amount = 1000;

    let transfer_ix = system_instruction::transfer(&info.swig_wallet_address, &recipient, amount);

    // Secp256r1 signing automatically handles replay protection
    let signature = loaded_wallet.sign_v2(vec![transfer_ix], None)?;
    println!("Transfer signed! Signature: {}", signature);

    // =========================================================================
    // 7. ADD SECP256R1 AUTHORITY TO EXISTING WALLET
    // =========================================================================
    println!("\n=== Adding Secp256r1 Authority ===");

    let (_, new_public_key) = create_secp256r1_keypair();
    println!("New authority: {}", hex::encode(&new_public_key));

    let permissions = vec![Permission::Sol {
        amount: 500_000_000,
        recurring: None,
    }];

    let sig =
        loaded_wallet.add_authority(AuthorityType::Secp256r1, &new_public_key, permissions)?;
    println!("Authority added! Signature: {}", sig);

    // =========================================================================
    // WebAuthn Integration Notes
    // =========================================================================
    println!("\n=== WebAuthn Integration Notes ===");
    println!("In a real WebAuthn implementation:");
    println!("1. Public key comes from navigator.credentials.create()");
    println!("2. Signing is done via navigator.credentials.get()");
    println!("3. The authenticator (hardware key, Touch ID, etc.) handles keys");
    println!();
    println!("This example uses software keys for demonstration.");

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

/// Create a secp256r1 (P-256) keypair using OpenSSL.
/// Returns the private key and 33-byte compressed public key.
fn create_secp256r1_keypair() -> (EcKey<Private>, [u8; 33]) {
    // Use the P-256 curve (also known as prime256v1 or secp256r1)
    let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();

    // Generate the private key
    let signing_key = EcKey::generate(&group).unwrap();

    // Extract the compressed public key (33 bytes)
    let mut ctx = BigNumContext::new().unwrap();
    let pubkey_bytes = signing_key
        .public_key()
        .to_bytes(&group, PointConversionForm::COMPRESSED, &mut ctx)
        .unwrap();

    let pubkey_array: [u8; 33] = pubkey_bytes.try_into().unwrap();
    (signing_key, pubkey_array)
}

/// Create a Secp256r1ClientRole with the signing function.
/// Uses the Solana secp256r1 program's sign_message for compatibility.
fn create_secp256r1_client_role(
    public_key: [u8; 33],
    private_key_der: Vec<u8>,
) -> Result<Secp256r1ClientRole, Box<dyn std::error::Error>> {
    use solana_secp256r1_program::sign_message;

    // The signing function takes a message hash (32 bytes) and returns a 64-byte signature (r || s)
    let signing_fn = Box::new(move |message_hash: &[u8]| -> [u8; 64] {
        sign_message(message_hash, &private_key_der).expect("Failed to sign message")
    });

    Ok(Secp256r1ClientRole::new(public_key, signing_fn))
}
