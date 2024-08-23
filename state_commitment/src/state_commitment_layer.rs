use std::collections::HashMap;
use std::str::FromStr;
use merkle::merkle_proof::MerkleProof;
use merkle::merkle_tree::MerkleTree;
use state::state_record::{StateRecord, ZkProof, ZkProofCommitment};

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

/// Represents a state commitment layer.
///
/// This layer encapsulates the state and the merkle tree data structure used to store the state.
/// The state is stored as key-value pairs in a hashmap, with each key represented as a `Vec<u8>` and each value as a `T` which implements the `StateRecord` trait.
///
/// # Generic Parameters
///
/// * `T`: The type of state record that implements the `StateRecord` trait.
///
/// # Fields
///
/// * `state`: A hashmap that stores the state as key-value pairs.
/// * `merkle_tree`: A merkle tree data structure used to store the state.
///
/// # Examples
///
/// ```rust
/// use std::collections::HashMap;
///
/// pub struct StateCommitmentLayer<T: StateRecord> {
///     state: HashMap<Vec<u8>, T>,
///     merkle_tree: MerkleTree<T>,
/// }
///
/// // Create a new state commitment layer
/// let state_commitment_layer = StateCommitmentLayer {
///     state: HashMap::new(),
///     merkle_tree: MerkleTree::new(),
/// };
/// ```
pub struct StateCommitmentLayer<T: StateRecord> {
    state: HashMap<Vec<u8>, T>,
    merkle_tree: MerkleTree<T>,
}

impl<T: StateRecord> StateCommitmentLayer<T> {
    pub fn new(state_records: Vec<T>) -> Self {
        let records_map = state_records.iter().map(|a| (a.get_key().to_vec(), a.clone())).collect();
        let merkle_tree = MerkleTree::new(state_records);
        StateCommitmentLayer { state: records_map, merkle_tree }
    }

    pub fn get_state_root(&self) -> Option<&[u8]> {
        self.merkle_tree.root_hash()
    }

    pub fn generate_proof(&self, key: &[u8]) -> Option<MerkleProof> {
        self.state.get(key).map(|account| {
            self.merkle_tree.generate_proof(account)
        })?
    }

    pub fn update_record(&mut self, state_record: T) {
        self.state.insert((*state_record.get_key()).to_owned(), state_record.clone());
        let records: Vec<_> = self.state.values().cloned().collect();
        self.merkle_tree = MerkleTree::new(records);
    }

    /// Verifies the ZK proof and signs the commitment.
    ///
    /// # Arguments
    ///
    /// * `zk_proof` - The ZK proof to verify.
    /// * `new_state_root` - The new state root.
    /// * `timestamp` - The timestamp.
    /// * `verifier_secret_key` - The verifier's secret key.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the `ZkProofCommitment` if verification and signing succeed,
    /// or a `Box<dyn std::error::Error>` if an error occurs.
    ///
    /// # Remarks
    ///
    /// This function verifies the provided ZK proof and signs the commitment if verification succeeds.
    /// The verification process is not included and will be implemented as a complex operation in the future.
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::ZkProof;
    ///
    /// let zk_proof = ZkProof::new();
    /// let new_state_root = [0u8; 32];
    /// let timestamp = 1234567890;
    /// let verifier_secret_key = [0u8; 32];
    ///
    /// let result = verify_and_sign(&zk_proof, new_state_root, timestamp, &verifier_secret_key);
    /// assert!(result.is_ok());
    /// ```
    fn verify_and_sign(
        &self,
        zk_proof: &ZkProof,
        new_state_root: [u8; 32],
        timestamp: u64,
        verifier_secret_key: &[u8; 32],
    ) -> Result<ZkProofCommitment, Box<dyn std::error::Error>> {
        // TODO
        // Verify the ZK proof (this would be a complex operation)
        // if !(self.verify_proof(zk_proof)) {
        //     return Err("ZK proof verification failed".into());
        // }

        // If verification succeeds, create and sign the commitment
        // let message = [&new_state_root[..]].concat();
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
            proof_hash: zk_proof.hash_sha256(),
            new_state_root,
            timestamp,
            verifier_signature: combined_signature,
            public_key,
        })
    }

    /// Commit the given ZkProof to Layer 1 (Solana blockchain).
    ///
    /// # Arguments
    ///
    /// * `zk_proof` - A reference to the ZkProof that will be committed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use my_crate::commit_to_l1;
    /// # use my_crate::ZkProof;
    /// # async fn example() {
    /// # let zk_proof = ZkProof::new();
    /// commit_to_l1(&zk_proof).await;
    /// # }
    /// ```
    pub async fn commit_to_l1(&self, zk_proof: &ZkProof) {
        // Connect to the Solana localnet
        let rpc_url = "http://127.0.0.1:8899".to_string();
        let client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

        // Load your Solana wallet keypair
        let payer = Keypair::new();
        let airdrop_amount = 1_000_000_000; // 1 SOL in lamports
        match StateCommitmentLayer::<T>::request_airdrop(&client, &payer.pubkey(), airdrop_amount).await {
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
        let state_root = self.get_state_root().expect("No state root available.");
        let r: [u8; 32] = state_root.try_into().unwrap();
        let secret = SecretKey::default().serialize();

        let commitment = self.verify_and_sign(
            &zk_proof,
            r.clone(),
            timestamp,
            &secret).unwrap(); //.unwrap()).expect("Error creating ZkProofCommitment");

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

}