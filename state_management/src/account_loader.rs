//! PayTube's "account loader" component, which provides the SVM API with the
//! ability to load accounts for PayTube channels.
//!
//! The account loader is a simple example of an RPC client that can first load
//! an account from the base chain, then cache it locally within the protocol
//! for the duration of the channel.

use std::str::FromStr;
use solana_client::rpc_client::RpcClient;
use solana_sdk::account::Account;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::native_loader;
use {
    solana_sdk::{
        account::{AccountSharedData, ReadableAccount},
        pubkey::Pubkey,
    },
    solana_svm::transaction_processing_callback::TransactionProcessingCallback,
    std::{collections::HashMap, sync::RwLock},
};
use log::debug;
use state::account_state::AccountState;
use crate::state_management::{ManageState, StateManager};

/// An account loading mechanism to hoist accounts from the base chain up to
/// an active PayTube channel.
///
/// Employs a simple cache mechanism to ensure accounts are only loaded once.
pub struct TrollupAccountLoader<'a, A: ManageState> {
    cache: RwLock<HashMap<[u8; 32], AccountSharedData>>,
    account_state_management: &'a StateManager<A>,
    rpc_client: RpcClient,
}

impl<'a, A: ManageState<Record=AccountState>> TrollupAccountLoader<'a, A> {
    pub fn new(account_state_management: &'a StateManager<A>) -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            account_state_management,
            rpc_client: RpcClient::new_with_commitment("http://127.0.0.1:8899".to_string(), CommitmentConfig::confirmed())
        }
    }
}

/// SVM implementation of the `AccountLoader` plugin trait.
///
/// In the Agave validator, this implementation is `Bank`.
impl <'a, A: ManageState<Record=AccountState>>  TransactionProcessingCallback for TrollupAccountLoader<'a, A> {
    fn account_matches_owners(&self, account: &Pubkey, owners: &[Pubkey]) -> Option<usize> {
        self.get_account_shared_data(account)
            .and_then(|account| owners.iter().position(|key| account.owner().eq(key)))
    }

    fn get_account_shared_data(&self, pubkey: &Pubkey) -> Option<AccountSharedData> {
        debug!("Getting shared account for {:?}", pubkey);
        if let Some(account) = self.cache.read().unwrap().get(&pubkey.clone().to_bytes()) {
            debug!("Found in cache... shared account for {:?}", pubkey);
            return Some(account.clone().into());
        }

        let client = RpcClient::new_with_commitment("https://api.devnet.solana.com".to_string(), CommitmentConfig::confirmed());
        let account = client.get_account(pubkey).ok();
        match account {
            None => {
                debug!("Not found... shared account for {:?}", pubkey);
                let asd: AccountSharedData = AccountSharedData::new(10000000000000, 0, &Pubkey::from_str("11111111111111111111111111111111").unwrap());
                self.cache.write().unwrap().insert(pubkey.to_bytes(), asd.clone());

                return Some(asd)
            }
            Some(account) => {
                debug!("Found in dev... shared account for {:?}", pubkey);

                let account_shared: AccountSharedData = account.clone().into();
                self.cache.write().unwrap().insert(pubkey.to_bytes(), account_shared.clone());

                return Some(account_shared)}
        }
    }

    fn add_builtin_account(&self, name: &str, program_id: &Pubkey) {
        let account_data = native_loader::create_loadable_account_with_fields(name, (5000, 0));

        self.cache
            .write()
            .unwrap()
            .insert(program_id.to_bytes(), account_data);
    }
}