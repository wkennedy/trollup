use std::str::FromStr;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use ark_bn254::{Bn254, Fq, Fq2, Fr, G1Affine, G2Affine};
use ark_ff::{Field, PrimeField};
use ark_groth16::{prepare_verifying_key, Groth16, ProvingKey, VerifyingKey, verifier, Proof};
use ark_relations::lc;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError, Variable};
use ark_relations::r1cs::ConstraintSystemRef::CS;
use ark_serialize::{CanonicalSerialize, SerializationError};
use ark_snark::SNARK;
use ark_std::{rand::thread_rng, One, UniformRand};
use light_poseidon::{Poseidon, PoseidonHasher};
use borsh::{BorshSerialize, BorshDeserialize, to_vec};
use base64::{encode, decode};


#[derive(BorshSerialize)]
enum ProgramInstruction {
    Initialize
}

const PROOF_VERIFIER_PROGRAM_ID: &str = "F68FK2Ai4vWVqFQpfx6RJjzpYieSzxWMqs179SBdcZVJ";
const SIGNATURE_VERIFIER_PROGRAM_ID: &str =  "7xyXvzfXcBhc8Tbv5gJp7j3XKzPaS3xEXGfwuDJ6MgAo";

#[tokio::main]
async fn main() {
    // Connect to the Solana devnet
    let rpc_url = "http://127.0.0.1:8899".to_string();
    // let rpc_url = "https://api.devnet.solana.com".to_string();
    let client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());
    
    // Load or create a keypair for the payer
    let payer = Keypair::new();
    let airdrop_amount = 1_000_000_000; // 1 SOL in lamports
    match request_airdrop(&client, &payer.pubkey(), airdrop_amount).await {
        Ok(_) => println!("Airdrop successful!"),
        Err(err) => eprintln!("Airdrop failed: {}", err),
    }

    // Define the program ID (replace with your actual program ID)
    let program_id = Pubkey::from_str(SIGNATURE_VERIFIER_PROGRAM_ID).unwrap(); // Replace with your actual program ID

    // Derive the PDA (Program Derived Address)
    let (pda, _) = Pubkey::find_program_address(&[b"state"], &program_id);

    // Create the instruction data
    let instruction_data = to_vec(&ProgramInstruction::Initialize).unwrap();

    // Create the instruction
    let instruction = Instruction::new_with_bytes(
        program_id,
        &instruction_data,
        vec![
            AccountMeta::new(pda, false),  // PDA account (writable, not signer)
            AccountMeta::new(payer.pubkey(), true),  // Payer account (writable, signer)
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),  // System program
        ],
    );

    // Get recent blockhash
    let recent_blockhash = client.get_latest_blockhash().await.unwrap();

    // Create transaction
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    // Send and confirm transaction
    let signature = client.send_and_confirm_transaction(&transaction).await.unwrap();
    println!("Initialization transaction sent successfully!");
    println!("Transaction signature: {}", signature);
    
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