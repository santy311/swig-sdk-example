//! Swig SDK Examples
//!
//! This crate contains standalone examples demonstrating the Swig SDK.
//! Each example can be run independently with `cargo run --example <name>`.
//!
//! Available examples:
//!
//! - `ed25519_wallet`: Create, load, and sign with Ed25519 (Solana native) authority
//! - `secp256k1_wallet`: Create, load, and sign with Secp256k1 (Ethereum) authority
//! - `secp256r1_wallet`: Create, load, and sign with Secp256r1 (WebAuthn) authority
//! - `wallet_operations`: Advanced operations (permissions, authorities, sub-accounts)
//! - `multi_wallet_manager`: Batch operations across multiple wallets
//!
//! # Quick Start
//!
//! ```bash
//! # Run the Ed25519 example
//! cargo run --example ed25519_wallet
//!
//! # Run the Secp256k1 (Ethereum) example
//! cargo run --example secp256k1_wallet
//!
//! # Run the Secp256r1 (WebAuthn) example
//! cargo run --example secp256r1_wallet
//!
//! # Run advanced wallet operations example
//! cargo run --example wallet_operations
//!
//! # Run multi-wallet batch operations example
//! cargo run --example multi_wallet_manager
//! ```

fn main() {
    println!("Swig SDK Examples");
    println!("=================");
    println!();
    println!("Available examples:");
    println!();
    println!("  cargo run --example ed25519_wallet");
    println!("    Create, load, and sign with Ed25519 (Solana native) authority");
    println!();
    println!("  cargo run --example secp256k1_wallet");
    println!("    Create, load, and sign with Secp256k1 (Ethereum) authority");
    println!();
    println!("  cargo run --example secp256r1_wallet");
    println!("    Create, load, and sign with Secp256r1 (WebAuthn/Passkey) authority");
    println!();
    println!("  cargo run --example wallet_operations");
    println!("    Advanced operations: permissions, authorities, sub-accounts, sessions");
    println!();
    println!("  cargo run --example multi_wallet_manager");
    println!("    Batch operations across multiple wallets");
}
