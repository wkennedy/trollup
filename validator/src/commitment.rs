use crate::error::ValidationError;
use crate::error::ValidationError::CommitmentTransactionFailed;
use crate::error::ValidationError::ProofVerificationFailed;
use ark_serialize::CanonicalSerializeHashExt;
use borsh::{to_vec, BorshDeserialize, BorshSerialize};
use lazy_static::lazy_static;
use libsecp256k1::{Message, PublicKey, SecretKey};
use log::info;
use sha2::Sha256;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program::keccak;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use state::config::TrollupConfig;
use state::state_record::{ZkProofCommitment};
use std::str::FromStr;
use serde_json::{json, Value};
use trollup_zk::prove::{ProofPackage, ProofPackagePrepared};
use trollup_zk::verify::verify_proof_package;
use crate::models::ApiResponse;

lazy_static! {
    static ref CONFIG: TrollupConfig = TrollupConfig::build().unwrap();
}

lazy_static! {
}

#[derive(BorshSerialize, BorshDeserialize)]
pub enum ProgramInstruction {
    Initialize,
    VerifySig(ZkProofCommitment),
}

fn create_and_sign_commitment(
    new_state_root: [u8; 32],
    verifier_secret_key: &[u8; 32],
) -> Result<ZkProofCommitment, Box<dyn std::error::Error>> {
    let message_hash = {
        let mut hasher = keccak::Hasher::default();
        hasher.hash(&new_state_root);
        hasher.result()
    };

    // If verification succeeds, create and sign the commitment
    let message = Message::parse_slice(&message_hash.0)?;

    // Create secret key from input bytes
    let secret_key = SecretKey::parse(verifier_secret_key)?;
    let public_key = PublicKey::from_secret_key(&secret_key).serialize();

    // Sign the message
    let (signature, recovery_id) = libsecp256k1::sign(&message, &secret_key);

    // Combine signature and recovery ID into 64 bytes
    let mut signature_bytes = [0u8; 64];
    signature_bytes[..64].copy_from_slice(&signature.serialize());

    Ok(ZkProofCommitment {
        verifier_signature: signature_bytes,
        recovery_id: recovery_id.serialize(),
        public_key,
        new_state_root,
    })
}

pub async fn verify_and_commit(proof_package_prepared: ProofPackagePrepared, new_state_root: [u8; 32]) -> Result<ApiResponse, ValidationError> {
    let client = RpcClient::new_with_commitment(CONFIG.rpc_url_current_env().to_string(), CommitmentConfig::confirmed());

    let proof_package: ProofPackage = proof_package_prepared.into();
    let is_valid = verify_proof_package(&proof_package);

    info!("Proof is valid. Creating commitment.");

    if !is_valid {
        return Err(ProofVerificationFailed);
    }

    // TODO thinking about using these for on chain data and/or logging...
    let proof = proof_package.proof;
    let hash: [u8; 32] = proof.hash::<Sha256>().into();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Load your Solana wallet keypair
    let payer = Keypair::from_bytes(&CONFIG.trollup_api_keypair).expect("Error loading API keypair");

    // Your program ID (replace with your actual program ID)
    let program_id = Pubkey::from_str(&CONFIG.signature_verifier_program_id).expect("");

    // Create and sign the commitment (this would normally be done by the trusted off-chain verifier)
    // TODO create and load this from somewhere else
    let secret = SecretKey::default().serialize();

    //TODO update to call specific instruction and call initialize
    let commitment = create_and_sign_commitment(
        new_state_root,
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
    let instruction_data = to_vec(&ProgramInstruction::VerifySig(commitment)).unwrap();
    let (pda, bump_seed) = Pubkey::find_program_address(&[b"state"], &program_id);
    let instruction = Instruction::new_with_bytes(
        program_id,
        instruction_data.as_slice(),
        vec![
            AccountMeta::new(pda, false),  // PDA account (writable, not signer)
        ],
    );

    // Create and send the transaction
    let recent_blockhash = client.get_latest_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    // Send and confirm transaction
    match client.send_and_confirm_transaction(&transaction).await {
        Ok(signature) => {
            info!("Transaction succeeded: {:?}", &signature);
            let response = ApiResponse {
                success: true,
                signature,
            };
            Ok(response)
        }
        Err(err) => {
            info!("Error sending transaction: {}", err);
            Err(CommitmentTransactionFailed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libsecp256k1::{PublicKey, SecretKey};
    use rand::thread_rng;

    #[test]
    fn test_create_and_sign_commitment() {
        let rng = &mut thread_rng();

        // Create test inputs
        let proof_hash = [1u8; 32];
        let new_state_root = [2u8; 32];
        let timestamp = 1632825600; // Example timestamp

        // Generate a test secret key
        let secret_key = SecretKey::default();
        let secret_key_bytes = secret_key.serialize();
        // let secret_key = SecretKey::random(&mut rng);
        // let secret_key_bytes = secret_key.serialize();

        // Call the function
        let result = create_and_sign_commitment(
            new_state_root,
            &secret_key_bytes,
        );

        // Assert that the result is Ok
        assert!(result.is_ok());

        // Unwrap the result
        let commitment = result.unwrap();

        // Verify the fields of the commitment
        // assert_eq!(commitment.proof_hash, proof_hash);
        assert_eq!(commitment.new_state_root, new_state_root);
        // assert_eq!(commitment.timestamp, timestamp);

        // Verify the public key
        let expected_public_key = PublicKey::from_secret_key(&secret_key).serialize();
        assert_eq!(commitment.public_key, expected_public_key);

        // Verify the signature
        let message = Message::parse_slice(&new_state_root).unwrap();
        let signature = libsecp256k1::Signature::parse_standard_slice(&commitment.verifier_signature[..64]).unwrap();
        assert!(libsecp256k1::verify(&message, &signature, &PublicKey::parse(&commitment.public_key).unwrap()));
    }
}