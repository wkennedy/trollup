use lazy_static::lazy_static;
use sha2::{Digest, Sha256};
use state::transaction::{convert_to_solana_transaction, TrollupTransaction};
use state_management::state_management::{ManageState, StateManager};
use std::sync::Arc;
use warp::{reply::json, Rejection, Reply};
use state::config::TrollupConfig;

type Result<T> = std::result::Result<T, Rejection>;

lazy_static! {
    static ref CONFIG: TrollupConfig = TrollupConfig::build().unwrap();
}

pub struct TransactionHandler<T: ManageState<Record=TrollupTransaction>> {
    transaction_state_management: Arc<StateManager<T>>,
}

impl <T: ManageState<Record=TrollupTransaction>> TransactionHandler<T> {
    pub fn new(transaction_state_management: Arc<StateManager<T>>) -> Self {
        TransactionHandler { transaction_state_management }
    }

    pub async fn get_transaction(&self, signature: &str) -> Result<impl Reply> {
        let hash: [u8; 32] = Sha256::digest(signature.as_bytes()).into();
        let option = self.transaction_state_management.get_state_record(&hash);
        match option {
            None => {
                Ok(json(&format!("No transaction found for: {:?}", signature)))
            }
            Some(transaction) => {
                Ok(json(&format!("Transaction details: {:?}", transaction)))
            }
        }
    }

    pub async fn get_all_transactions(&self) -> Result<impl Reply> {
        let transactions = self.transaction_state_management.get_all_entries();
        let mut solana_txs = Vec::with_capacity(transactions.len());
        for (_, trollup_transaction) in transactions {
            solana_txs.push(convert_to_solana_transaction(trollup_transaction).expect("TODO: panic message"));
        }
        Ok(json(&solana_txs))
    }
    
    pub async fn challenge(&self) -> Result<impl Reply> {
        let transactions = self.transaction_state_management.get_all_entries();
        let mut solana_txs = Vec::with_capacity(transactions.len());
        for (_, trollup_transaction) in transactions {
            solana_txs.push(convert_to_solana_transaction(trollup_transaction).expect("TODO: panic message"));
        }
        Ok(json(&solana_txs))
    }
}
