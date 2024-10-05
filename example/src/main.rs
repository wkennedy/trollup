use std::time::Duration;
use reqwest::Client;
use anyhow::Result;
use lazy_static::lazy_static;
use log::info;
use solana_program::hash::Hash;
use solana_program::instruction::CompiledInstruction;
use solana_program::message::{Message, MessageHeader};
use solana_program::pubkey::Pubkey;
use solana_program::system_instruction;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use state::config::TrollupConfig;

const BASE_URL: &str = "http://localhost:27182";

lazy_static! {
    static ref CONFIG: TrollupConfig = TrollupConfig::build().unwrap();
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

    let send_result = client.send_transaction_optimistic(&transaction).await?;
    println!("Send transaction result: {}", send_result);

    // Get transaction details
    let signature = "your_transaction_signature_here";
    let transaction_details = client.get_transaction(signature).await?;
    println!("Transaction details: {}", transaction_details);

    tokio::time::sleep(Duration::from_secs(3)).await;

    let account = client.get_all_accounts().await?;
    println!("Account details: {}", account);
    
    let block = client.get_all_blocks().await?;
    println!("Block details: {}", block);
    
    let transactions = client.get_all_transactions().await?;
    println!("Transactions details: {}", transactions);
    
    Ok(())
}