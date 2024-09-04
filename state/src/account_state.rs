use crate::state_record::StateRecord;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// Represents the state of an account.
#[derive(Debug, BorshDeserialize, BorshSerialize, Clone)]
pub struct AccountState {
    pub address: Pubkey,
    pub balance: u64,
    pub nonce: u64
}

impl StateRecord for AccountState {
    fn get_key(&self) -> Option<[u8; 32]> {
        Some(self.address.to_bytes())
    }
}