use std::collections::HashMap;
use std::str::FromStr;
use state::state_record::{StateRecord, ZkProof, ZkProofCommitment};
use ark_serialize::CanonicalSerializeHashExt;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use borsh::{to_vec};
use libsecp256k1::{Message, PublicKey, SecretKey};
use sha2::Sha256;
use trollup_zk::prove::{ProofPackage, ProofPackagePrepared};
use trollup_zk::verify::{verify_prepared_proof_package, verify_proof_package};

fn create_and_sign_commmitment(
    proof_hash: [u8; 32],
    new_state_root: [u8; 32],
    timestamp: u64,
    verifier_secret_key: &[u8; 32],
) -> Result<ZkProofCommitment, Box<dyn std::error::Error>> {

    // If verification succeeds, create and sign the commitment
    let message = Message::parse_slice(&new_state_root)?;

    // Create secret key from input bytes
    let secret_key = SecretKey::parse(verifier_secret_key)?;
    let public_key = PublicKey::from_secret_key(&secret_key).serialize_compressed();

    // Sign the message
    let (signature, _recovery_id) = libsecp256k1::sign(&message, &secret_key);

    // Combine signature and recovery ID into 64 bytes
    let mut combined_signature = [0u8; 64];
    combined_signature[..64].copy_from_slice(&signature.serialize());
    // combined_signature[63] = recovery_id.serialize();

    Ok(ZkProofCommitment {
        proof_hash: proof_hash,
        new_state_root,
        timestamp,
        verifier_signature: combined_signature,
        public_key,
    })
}


pub async fn verify_and_commit(proof_package_prepared: ProofPackagePrepared, new_state_root: [u8; 32]) {
    // Connect to the Solana localnet
    let rpc_url = "http://127.0.0.1:8899".to_string();
    let client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    let proof_package: ProofPackage = proof_package_prepared.into();
    let is_valid = verify_proof_package(&proof_package);

    if !is_valid {
        // TODO return proof invalid, return don't finalize commit
        return
    }

    let proof = proof_package.proof;
    let hash: [u8; 32] = proof.hash::<Sha256>().into();

    // Load your Solana wallet keypair
    let payer = Keypair::new();
    let airdrop_amount = 1_000_000_000; // 1 SOL in lamports
    match request_airdrop(&client, &payer.pubkey(), airdrop_amount).await {
        Ok(_) => println!("Airdrop successful!"),
        Err(err) => eprintln!("Airdrop failed: {}", err),
    }

    // Your program ID (replace with your actual program ID)
    let program_id = Pubkey::from_str("3nMqU7dFciQJQyjjZj1Gh3Ctt5fhe6g7WUbqMXRjJhzB").expect("");

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Create and sign the commitment (this would normally be done by the trusted off-chain verifier)
    // TODO create and load this from somewhere else
    let secret = SecretKey::default().serialize();

    let commitment = create_and_sign_commmitment(
        hash,
        new_state_root,
        timestamp,
        &secret).unwrap();

    // Serialize the commitment
    let instruction_data = to_vec(&commitment).unwrap();

    // Calculate the exact size needed for the account
    let account_size = instruction_data.len();

    // Create the program account that will store the state
    let state_account = Keypair::new();
    let create_account_ix = solana_sdk::system_instruction::create_account(
        &payer.pubkey(),
        &state_account.pubkey(),
        client.get_minimum_balance_for_rent_exemption(account_size).await.unwrap(), // Size of the state (32 bytes)
        account_size as u64, // Size of the account data
        &program_id,
    );

    // Create the instruction to call our program
    let accounts = vec![AccountMeta::new(state_account.pubkey(), false)];
    let instruction = Instruction::new_with_borsh(
        program_id,
        &commitment,
        accounts,
    );

    // Create and send the transaction
    let recent_blockhash = client.get_latest_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[create_account_ix, instruction],
        Some(&payer.pubkey()),
        &[&payer, &state_account],
        recent_blockhash,
    );

    // Send and confirm transaction
    match client.send_and_confirm_transaction(&transaction).await {
        Ok(signature) => println!("Transaction sent successfully. Signature: {}", signature),
        Err(err) => println!("Error sending transaction: {}", err),
    }
}

async fn request_airdrop(client: &RpcClient, pubkey: &Pubkey, amount: u64) -> Result<(), Box<dyn std::error::Error>> {
    let signature = client.request_airdrop(pubkey, amount).await?;

    // Wait for the transaction to be confirmed
    loop {
        let confirmation = client.confirm_transaction(&signature).await.unwrap();
        if confirmation {
            break;
        }
    }
    Ok(())
}