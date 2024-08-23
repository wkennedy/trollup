use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;
use crate::state_record::{StateRecord, ZkProof};

/// Represents a block processed by this L2.
///
/// A block contains the following information:
/// - `id`: The identifier of the block.
/// - `transactions_merkle_root`: The root hash of the Merkle tree of transactions in the block.
/// - `transaction_zk_proof`: A zero-knowledge proof for the validity of transactions.
/// - `accounts_merkle_root`: The root hash of the Merkle tree of accounts in the block.
/// - `accounts_zk_proof`: A zero-knowledge proof for the validity of accounts.
/// - `transactions`: A vector of public keys representing the transactions in the block.
/// - `accounts`: A vector of public keys representing the accounts in the block.
#[derive(Debug, BorshDeserialize, BorshSerialize, Clone)]
pub struct Block {
    pub id: String,
    pub transactions_merkle_root: Box<[u8]>,
    pub transaction_zk_proof: ZkProof,
    pub accounts_merkle_root: Box<[u8]>,
    pub accounts_zk_proof: ZkProof,
    pub transactions: Vec<Pubkey>,
    pub accounts: Vec<Pubkey>
}

impl Block {
    pub fn new(block_number: u64, transactions_merkle_root: Box<[u8]>, transaction_zk_proof: ZkProof, accounts_merkle_root: Box<[u8]>, accounts_zk_proof: ZkProof, transactions: Vec<Pubkey>, accounts: Vec<Pubkey>) -> Self {
        Block {
            id: ["block_", block_number.to_string().as_str()].concat(),
            transactions_merkle_root,
            transaction_zk_proof,
            accounts_merkle_root,
            accounts_zk_proof,
            transactions,
            accounts,
        }
    }
}

impl StateRecord for Block {
    fn get_key(&self) -> &[u8] {
        self.id.as_ref()
    }
}
