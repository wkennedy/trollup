use anyhow::Result;
use ark_bn254::Bn254;
use ark_groth16::{Proof, VerifyingKey};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize, Compress};
use borsh::to_vec;
use borsh_derive::{BorshDeserialize, BorshSerialize};
use lazy_static::lazy_static;
use log::info;
use reqwest::Client;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program::alt_bn128::compression::prelude::convert_endianness;
use solana_program::hash::Hash;
use solana_program::instruction::{AccountMeta, CompiledInstruction, Instruction};
use solana_program::message::{Message, MessageHeader};
use solana_program::pubkey::Pubkey;
use solana_program::system_instruction;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use state::account_state::AccountState;
use state::config::TrollupConfig;
use state::state_record::{StateCommitmentPackage, StateCommitmentPackageUI};
use std::ops::Neg;
use std::str::FromStr;
use std::time::Duration;
use tokio::fs;
use trollup_zk::verify_lite::{convert_arkworks_vk_to_solana_example, Groth16VerifierPrepared, Groth16VerifyingKeyPrepared, ProofCommitmentPackage};

const BASE_URL: &str = "http://localhost:27182";

lazy_static! {
    static ref CONFIG: TrollupConfig = TrollupConfig::build().unwrap();
}

#[derive(BorshSerialize, BorshDeserialize)]
pub enum ProgramInstruction {
    Initialize,
    VerifyProof(ProofCommitmentPackage),
}

struct TrollupClient {
    client: Client,
}

impl TrollupClient {
    fn new() -> Self {
        TrollupClient {
            client: Client::new(),
        }
    }

    async fn health_check(&self) -> Result<String> {
        let response = self.client
            .get(format!("{}/health", BASE_URL))
            .send()
            .await?;

        Ok(response.text().await?)
    }

    async fn send_transaction(&self, transaction: &Transaction) -> Result<String> {
        let response = self.client
            .post(format!("{}/send-transaction", BASE_URL))
            .json(transaction)
            .send()
            .await?;

        Ok(response.text().await?)
    }

    async fn send_transaction_optimistic(&self, transaction: &Transaction) -> Result<String> {
        let response = self.client
            .post(format!("{}/send-transaction-optimistic", BASE_URL))
            .json(transaction)
            .send()
            .await?;

        Ok(response.text().await?)
    }

    async fn get_transaction(&self, signature: &str) -> Result<String> {
        let response = self.client
            .get(format!("{}/get-transaction/{}", BASE_URL, signature))
            .send()
            .await?;

        Ok(response.text().await?)
    }

    async fn get_account(&self, account_id: &str) -> Result<String> {
        let response = self.client
            .get(format!("{}/get-account/{}", BASE_URL, account_id))
            .send()
            .await?;

        Ok(response.text().await?)
    }

    async fn get_latest_block(&self) -> Result<String> {
        let response = self.client
            .get(format!("{}/get-latest-block/", BASE_URL))
            .send()
            .await?;

        Ok(response.text().await?)
    }

    async fn get_block(&self, block_id: u64) -> Result<String> {
        let response = self.client
            .get(format!("{}/get-block/{}", BASE_URL, block_id))
            .send()
            .await?;

        Ok(response.text().await?)
    }

    async fn get_all_transactions(&self) -> Result<String> {
        let response = self.client
            .get(format!("{}/get-all-transactions/", BASE_URL))
            .send()
            .await?;

        Ok(response.text().await?)
    }

    async fn get_all_accounts(&self) -> Result<String> {
        let response = self.client
            .get(format!("{}/get-all-accounts/", BASE_URL))
            .send()
            .await?;

        Ok(response.text().await?)
    }

    async fn get_all_pending_commits(&self) -> Result<String> {
        let response = self.client
            .get(format!("{}/get-all-pending-commitments/", BASE_URL))
            .send()
            .await?;

        // let json = response.text().await?;
        // let v: Value = serde_json::from_str(&json)?;
        // let formatted = serde_json::to_string_pretty(&v)?;
        //
        // Ok(formatted)
        Ok(response.text().await?)
    }

    async fn get_all_pending_commits_full(&self) -> Result<Vec<StateCommitmentPackageUI<AccountState>>> {
        let response = self.client
            .get(format!("{}/get-all-pending-commitments/", BASE_URL))
            .send()
            .await?;

        // let json = response.text().await?;
        // let v: Value = serde_json::from_str(&json)?;
        // let formatted = serde_json::to_string_pretty(&v)?;
        //
        // Ok(formatted)
        Ok(response.json::<Vec<StateCommitmentPackageUI<AccountState>>>().await?)
    }

    async fn get_all_blocks(&self) -> Result<String> {
        let response = self.client
            .get(format!("{}/get-all-blocks/", BASE_URL))
            .send()
            .await?;

        Ok(response.text().await?)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = TrollupConfig::load();

    let client = TrollupClient::new();

    // Health check
    let health_status = client.health_check().await?;
    info!("Health status: {}", health_status);

    let sender = Keypair::new();

    // Specify the recipient's public key
    let recipient = Pubkey::new_unique();

    // Amount to transfer (in lamports)
    let amount = 1_000_000; // 0.001 SOL

    // Create the transfer instruction
    let instruction = system_instruction::transfer(
        &sender.pubkey(),
        &recipient,
        amount,
    );

    // Create a TrollupTransaction
    let transaction = Transaction {
        signatures: vec![sender.sign_message(&[0u8; 32]).into()], // Placeholder signature
        message: Message {
            header: MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 1,
            },
            account_keys: vec![sender.pubkey(), recipient, solana_sdk::system_program::id()],
            recent_blockhash: Hash::default(),
            instructions: vec![CompiledInstruction {
                program_id_index: 2, // Index of the system program in account_keys
                accounts: vec![0, 1], // Indices of sender and recipient in account_keys
                data: instruction.data.clone(),
            }],
        },
    };

    let send_result = client.send_transaction(&transaction).await?;
    println!("Send transaction result: {}", send_result);

    // Get transaction details
    let signature = "your_transaction_signature_here";
    let transaction_details = client.get_transaction(signature).await?;
    println!("Transaction details: {}", transaction_details);
    // 
    // tokio::time::sleep(Duration::from_secs(3)).await;
    // 
    // let account = client.get_all_accounts().await?;
    // println!("Account details: {}", account);
    // 
    // let block = client.get_all_blocks().await?;
    // println!("Block details: {}", block);
    // 
    // let transactions = client.get_all_transactions().await?;
    // println!("Transactions details: {}", transactions);
    // 
    // let pending_commits = client.get_all_pending_commits().await?;
    // println!("Pending commits: {}", pending_commits);
    // 
    // let rpc_client = RpcClient::new_with_commitment(CONFIG.rpc_url_current_env().to_string(), CommitmentConfig::confirmed());
    // 
    // let commitment_packages = client.get_all_pending_commits_full().await.expect("TODO: panic message");
    // 
    // let payer = Keypair::from_bytes(&CONFIG.trollup_api_keypair)?;
    // // let payer = Keypair::new();
    // // let airdrop_amount = 1_000_000; // 1 SOL in lamports
    // // match request_airdrop(&rpc_client, &payer.pubkey(), airdrop_amount).await {
    // //     Ok(_) => println!("Airdrop successful!"),
    // //     Err(err) => eprintln!("Airdrop failed: {}", err),
    // // }
    // 
    // for commitment_package in commitment_packages {
    //     let verifier_prepared = build_verifier(commitment_package.proof, commitment_package.public_inputs, commitment_package.verifying_key);
    //     let proof_commitment_package = ProofCommitmentPackage {
    //         groth16_verifier_prepared: verifier_prepared,
    //         state_root: commitment_package.state_root.unwrap(),
    //     };
    //     // Serialize and encode the proof package
    //     // let serialized_proof = to_vec(&proof_commitment_package).unwrap();
    //     let program_id = Pubkey::from_str(&CONFIG.proof_verifier_program_id)?;
    //     let instruction_data = to_vec(&ProgramInstruction::VerifyProof(proof_commitment_package)).unwrap();
    //     let (pda, bump_seed) = Pubkey::find_program_address(&[b"state"], &program_id);
    //     let instruction = Instruction::new_with_bytes(
    //         program_id,
    //         instruction_data.as_slice(),
    //         vec![
    //             AccountMeta::new(pda, false),  // PDA account (writable, not signer)
    //         ],
    //     );
    // 
    //     // Create and send the transaction
    //     let recent_blockhash = rpc_client.get_latest_blockhash().await.unwrap();
    //     let transaction = Transaction::new_signed_with_payer(
    //         &[instruction],
    //         Some(&payer.pubkey()),
    //         &[&payer],
    //         recent_blockhash,
    //     );
    // 
    //     // Send and confirm transaction
    //     match rpc_client.send_and_confirm_transaction_with_spinner(&transaction).await {
    //         Ok(signature) => println!("Transaction succeeded! Signature: {}", signature),
    //         Err(err) => println!("Transaction failed: {:?}", err),
    //     }
    // 
    // }
    
    Ok(())
}

fn build_verifier(proof_bytes: Vec<u8>, public_inputs: Vec<u8>, verifying_key: Vec<u8>) -> Groth16VerifierPrepared {
    let proof = Proof::<Bn254>::deserialize_uncompressed_unchecked(proof_bytes.as_slice()).expect("Error deserializing proof");

    let proof_with_neg_a = Proof::<Bn254> {
        a: proof.a.neg(),
        b: proof.b,
        c: proof.c,
    };
    let mut proof_bytes = Vec::with_capacity(proof_with_neg_a.serialized_size(Compress::No));
    proof_with_neg_a.serialize_uncompressed(&mut proof_bytes).expect("Error serializing proof");

    let proof_a: [u8; 64] = convert_endianness::<32, 64>(proof_bytes[0..64].try_into().unwrap());
    let proof_b: [u8; 128] = convert_endianness::<64, 128>(proof_bytes[64..192].try_into().unwrap());
    let proof_c: [u8; 64] = convert_endianness::<32, 64>(proof_bytes[192..256].try_into().unwrap());
    
    let prepared_public_input = convert_endianness::<32, 64>(<&[u8; 64]>::try_from(public_inputs.as_slice()).unwrap());

    let vk = VerifyingKey::<Bn254>::deserialize_uncompressed_unchecked(verifying_key.as_slice()).expect("Error deserializing verifying key");

    let groth_vk = convert_arkworks_vk_to_solana_example(&vk);
    let groth_vk_prepared = Groth16VerifyingKeyPrepared {
        vk_alpha_g1: groth_vk.vk_alpha_g1,
        vk_beta_g2: groth_vk.vk_beta_g2,
        vk_gamma_g2: groth_vk.vk_gamma_g2,
        vk_delta_g2: groth_vk.vk_delta_g2,
    };

    let verifier: Groth16VerifierPrepared = Groth16VerifierPrepared::new(
        proof_a,
        proof_b,
        proof_c,
        prepared_public_input,
        Box::new(groth_vk_prepared),
    ).unwrap();
    verifier
}

async fn request_airdrop(client: &RpcClient, pubkey: &Pubkey, amount: u64) -> std::result::Result<(), Box<dyn std::error::Error>> {
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