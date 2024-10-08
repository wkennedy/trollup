use crate::state_record::StateRecord;
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use solana_sdk::account::{Account, AccountSharedData};
use solana_sdk::clock::Epoch;
use solana_sdk::pubkey::Pubkey;

/// Represents the state of an account.
#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, Serialize, Deserialize)]
pub struct AccountState {
    pub address: Pubkey,
    /// lamports in the account
    pub lamports: u64,
    /// data held in this account
    pub data: Vec<u8>,
    /// the program that owns this account. If executable, the program that loads this account.
    pub owner: Pubkey,
    /// this account's data contains a loaded program (and is now read-only)
    pub executable: bool,
    /// the epoch at which this account will next owe rent
    pub rent_epoch: Epoch,
}

impl StateRecord for AccountState {
    fn get_key(&self) -> [u8; 32] {
        self.address.to_bytes()
    }
}

impl From<AccountSharedData> for AccountState {
    fn from(other: AccountSharedData) -> Self {
        let account = Account::from(other);
        Self {
            address: Default::default(),
            lamports: account.lamports,
            data: account.data,
            owner: account.owner,
            executable: account.executable,
            rent_epoch: account.rent_epoch,
        }
    }
}

impl Into<AccountSharedData> for AccountState {
    fn into(self) -> AccountSharedData {
        let account = Account {
            lamports: self.lamports,
            data: self.data,
            owner: self.owner,
            executable: self.executable,
            rent_epoch: self.rent_epoch,
        };

        AccountSharedData::from(account)
    }
}