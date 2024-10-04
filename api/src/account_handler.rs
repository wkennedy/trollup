use crate::config::{Config, TrollupConfig};
use lazy_static::lazy_static;
use solana_sdk::pubkey::Pubkey;
use state::account_state::AccountState;
use state_management::state_management::{ManageState, StateManager};
use std::str::FromStr;
use std::sync::Arc;
use warp::{reply::json, Rejection, Reply};
use state::transaction::convert_to_solana_transaction;

type Result<T> = std::result::Result<T, Rejection>;

lazy_static! {
    static ref CONFIG: TrollupConfig = TrollupConfig::build().unwrap();
}

pub struct AccountHandler<A: ManageState<Record=AccountState>> {
    account_state_management: Arc<StateManager<A>>,
}

impl <A: ManageState<Record=AccountState>> AccountHandler<A> {
    pub fn new(account_state_management: Arc<StateManager<A>>) -> Self {
        AccountHandler { account_state_management }
    }

    pub async fn get_account(&self, account_id: &str) -> Result<impl Reply> {
        let pubkey = Pubkey::from_str(account_id).unwrap();
        let option = self.account_state_management.get_state_record(&pubkey.to_bytes());
        match option {
            None => {
                Ok(json(&format!("No account found for: {:?}", account_id)))
            }
            Some(account) => {
                Ok(json(&account))
            }
        }
    }

    pub async fn get_all_accounts(&self) -> Result<impl Reply> {
        let accounts = self.account_state_management.get_all_entries();
        Ok(json(&accounts))
    }
}