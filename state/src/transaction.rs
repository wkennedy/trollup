use crate::state_record::StateRecord;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;
use std::time::{SystemTime, UNIX_EPOCH};

/// This struct represents a transaction with the following fields:
/// - `id`: The public key of the transaction.
/// - `account`: The public key of the account associated with the transaction.
/// - `balance`: The balance associated with the transaction.
/// - `nonce`: The nonce value of the transaction.
/// - `timestamp`: The timestamp of the transaction.
#[derive(Debug, BorshDeserialize, BorshSerialize, Clone)]
pub struct Transaction {
    pub id: Pubkey,
    pub account: Pubkey,
    pub balance: u64,
    pub nonce: u64,
    pub timestamp: u128
}

impl Transaction {
    pub fn new(account: Pubkey, balance: u64, nonce: u64) -> Self {
        let start = SystemTime::now();
        let now = start
            .duration_since(UNIX_EPOCH).unwrap().as_millis();
        Self {
            id: Pubkey::new_unique(),
            account,
            balance,
            nonce,
            timestamp: now
        }
    }
}

impl StateRecord for Transaction {
    fn get_key(&self) -> Option<[u8; 32]> {
        Some(self.id.to_bytes())
    }
}
