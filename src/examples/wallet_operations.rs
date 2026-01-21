//! Wallet Operations Example
//!
//! This example demonstrates advanced wallet operations including:
//! - Authority management (add, update, remove)
//! - Permission types
//! - Sub-account operations
//! - Session management
//!
//! Run with: `cargo run --example wallet_operations`

use rand::Rng;
use solana_client::rpc_client::RpcClient;
use solana_keypair::Keypair;
use solana_sdk::{pubkey::Pubkey, system_instruction, transaction::Transaction};
use solana_signer::{EncodableKey, Signer};
use std::path::Path;
use swig_sdk::{
    authority::AuthorityType, types::UpdateAuthorityData, Ed25519ClientRole, Permission,
    RecurringConfig, SwigWallet,
};

const RPC_URL: &str = "https://api.devnet.solana.com";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let authority = load_or_create_keypair("authority.json");
    println!("Authority: {}", authority.pubkey());

    let rpc_client = RpcClient::new(RPC_URL);

    // Create a wallet for this example
    let swig_id: [u8; 32] = rand::thread_rng().r#gen();
    let mut wallet = SwigWallet::builder()
        .with_swig_id(swig_id)
        .with_client_role(Box::new(Ed25519ClientRole::new(authority.pubkey())))
        .with_rpc_url(RPC_URL.to_string())
        .with_fee_payer(&authority)
        .with_authority_keypair(Some(&authority))
        .create()?;

    println!("Wallet created: {}", hex::encode(swig_id));

    // =========================================================================
    // 1. PERMISSION TYPES
    // =========================================================================
    println!("\n=== Permission Types ===");
    demonstrate_permission_types();

    // =========================================================================
    // 2. ADD AUTHORITY WITH PERMISSIONS
    // =========================================================================
    println!("\n=== Adding Authorities ===");

    // Authority with full access
    let admin_key = Keypair::new();
    wallet.add_authority(
        AuthorityType::Ed25519,
        admin_key.pubkey().as_ref(),
        vec![Permission::All],
    )?;
    println!("Added admin authority: {}", admin_key.pubkey());

    // Authority with SOL transfer limit
    let spender_key = Keypair::new();
    wallet.add_authority(
        AuthorityType::Ed25519,
        spender_key.pubkey().as_ref(),
        vec![Permission::Sol {
            amount: 1_000_000_000, // 1 SOL
            recurring: None,
        }],
    )?;
    println!("Added spender authority: {}", spender_key.pubkey());

    // Authority with recurring SOL limit (resets periodically)
    let daily_spender = Keypair::new();
    wallet.add_authority(
        AuthorityType::Ed25519,
        daily_spender.pubkey().as_ref(),
        vec![Permission::Sol {
            amount: 100_000_000, // 0.1 SOL per window
            recurring: Some(RecurringConfig {
                window: 216_000, // ~1 day in slots (assuming 400ms slots)
                last_reset: 0,
                current_amount: 0,
            }),
        }],
    )?;
    println!("Added daily spender: {}", daily_spender.pubkey());

    // Authority with token permissions
    let token_manager = Keypair::new();
    let token_mint = Pubkey::new_unique(); // Replace with real mint
    wallet.add_authority(
        AuthorityType::Ed25519,
        token_manager.pubkey().as_ref(),
        vec![Permission::Token {
            mint: token_mint,
            amount: 1_000_000,
            recurring: None,
        }],
    )?;
    println!("Added token manager: {}", token_manager.pubkey());

    // =========================================================================
    // 3. VIEW ALL AUTHORITIES
    // =========================================================================
    println!("\n=== Current Authorities ===");

    let info = wallet.get_info()?;
    for role in &info.roles {
        println!("\nRole ID: {}", role.role_id);
        println!("  Type: {:?}", role.authority_type);
        println!("  Identity: {}", hex::encode(&role.authority_identity));
        println!("  Permissions:");
        for perm in &role.permissions {
            println!("    - {:?}", perm);
        }
    }

    // =========================================================================
    // 4. UPDATE AUTHORITY PERMISSIONS
    // =========================================================================
    println!("\n=== Updating Authority Permissions ===");

    // Find the spender's role ID
    let spender_role_id = info
        .roles
        .iter()
        .find(|r| r.authority_identity == spender_key.pubkey().as_ref())
        .map(|r| r.role_id)
        .expect("Spender role not found");

    // Add more permissions using AddActions
    wallet.update_authority(
        spender_role_id,
        UpdateAuthorityData::AddActions(vec![Permission::Sol {
            amount: 500_000_000, // Additional 0.5 SOL permission
            recurring: None,
        }]),
    )?;
    println!("Added permissions to role {}", spender_role_id);

    // Note: ReplaceAll is also available:
    // wallet.update_authority(
    //     spender_role_id,
    //     UpdateAuthorityData::ReplaceAll(vec![Permission::Sol {
    //         amount: 2_000_000_000, // 2 SOL total
    //         recurring: None,
    //     }]),
    // )?;

    // =========================================================================
    // 5. REMOVE AUTHORITY
    // =========================================================================
    println!("\n=== Removing Authority ===");

    wallet.remove_authority(daily_spender.pubkey().as_ref())?;
    println!("Removed daily spender authority");

    // Or remove by role ID
    // wallet.remove_authority_by_id(role_id)?;

    // =========================================================================
    // 6. FUND THE WALLET
    // =========================================================================
    println!("\n=== Funding Wallet ===");

    let info = wallet.get_info()?;
    let fund_amount = 10_000_000; // 0.01 SOL
    fund_wallet(&rpc_client, &authority, &info.swig_wallet_address, fund_amount)?;
    println!("Funded wallet with {} lamports", fund_amount);

    // =========================================================================
    // 7. SIGN TRANSACTIONS
    // =========================================================================
    println!("\n=== Signing Transactions ===");

    // Simple SOL transfer
    let transfer_ix =
        system_instruction::transfer(&info.swig_wallet_address, &Pubkey::new_unique(), 1000);
    let sig = wallet.sign_v2(vec![transfer_ix], None)?;
    println!("SOL transfer: {}", sig);

    // Multiple instructions in one transaction
    let ix1 =
        system_instruction::transfer(&info.swig_wallet_address, &Pubkey::new_unique(), 500);
    let ix2 =
        system_instruction::transfer(&info.swig_wallet_address, &Pubkey::new_unique(), 500);
    let sig = wallet.sign_v2(vec![ix1, ix2], None)?;
    println!("Multi-instruction tx: {}", sig);

    // =========================================================================
    // 8. SUB-ACCOUNT OPERATIONS (Advanced)
    // =========================================================================
    // Sub-accounts allow creating derived addresses from the main wallet.
    // This feature requires specific setup - see SDK documentation.
    //
    // Example (when properly configured):
    // let sub_account_sig = wallet.create_sub_account()?;
    // println!("Created sub-account: {}", sub_account_sig);
    //
    // let sub_account_ix = system_instruction::transfer(
    //     &info.swig_wallet_address,
    //     &Pubkey::new_unique(),
    //     100,
    // );
    // let sig = wallet.sign_with_sub_account(vec![sub_account_ix], None)?;

    // =========================================================================
    // 9. SESSION MANAGEMENT (Advanced)
    // =========================================================================
    // Sessions allow temporary delegation of signing authority.
    // This feature requires specific setup - see SDK documentation.
    //
    // Example (when properly configured):
    // let session_key = Keypair::new();
    // let session_duration = 3600; // slots
    // wallet.create_session(session_key.pubkey(), session_duration)?;

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

/// Demonstrate all permission types
fn demonstrate_permission_types() {
    println!("Available Permission Types:");
    println!();

    println!("1. Permission::All");
    println!("   Full unrestricted access to the wallet");
    println!();

    println!("2. Permission::ManageAuthority");
    println!("   Can add/remove/update other authorities");
    println!();

    println!("3. Permission::Sol {{ amount, recurring }}");
    println!("   Transfer SOL up to amount limit");
    println!();

    println!("4. Permission::SolDestination {{ destination, amount, recurring }}");
    println!("   Transfer SOL only to specific destination");
    println!();

    println!("5. Permission::Token {{ mint, amount, recurring }}");
    println!("   Transfer tokens of specific mint up to amount");
    println!();

    println!("6. Permission::TokenDestination {{ mint, destination, amount, recurring }}");
    println!("   Transfer tokens only to specific destination");
    println!();

    println!("7. Permission::Program {{ program_id }}");
    println!("   Execute specific program");
    println!();

    println!("8. Permission::ProgramAll");
    println!("   Execute any program");
    println!();

    println!("9. Permission::SubAccount {{ sub_account }}");
    println!("   Manage specific sub-account");
    println!();

    println!("10. Permission::Stake {{ amount, recurring }}");
    println!("    Stake operations up to amount");
    println!();

    println!("11. Permission::AllButManageAuthority");
    println!("    All permissions except managing authorities");
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
