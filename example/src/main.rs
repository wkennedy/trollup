use reqwest::Client;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use solana_program::hash::Hash;
use solana_program::instruction::CompiledInstruction;
use solana_program::message::{Message, MessageHeader};
use solana_program::pubkey::Pubkey;
use solana_program::system_instruction;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;

const BASE_URL: &str = "http://localhost:8080";

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

    async fn get_transaction(&self, signature: &str) -> Result<String> {
        let response = self.client
            .get(format!("{}/get-transaction/{}", BASE_URL, signature))
            .send()
            .await?;

        Ok(response.text().await?)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let client = TrollupClient::new();

    // Health check
    let health_status = client.health_check().await?;
    println!("Health status: {}", health_status);

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
            header:MessageHeader {
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

    Ok(())
}