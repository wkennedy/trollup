use base64::{engine::general_purpose, Engine as _};
use lazy_static::lazy_static;
use state::account_state::AccountState;
use state::config::TrollupConfig;
use state::state_record::{StateCommitmentPackage, StateCommitmentPackageUI};
use state_management::state_management::{ManageState, StateManager};
use std::sync::Arc;
use warp::{reply::json, Rejection, Reply};

type Result<T> = std::result::Result<T, Rejection>;

lazy_static! {
    static ref CONFIG: TrollupConfig = TrollupConfig::build().unwrap();
}

pub struct OptimisticHandler<T: ManageState<Record=StateCommitmentPackage<AccountState>>> {
    optimistic_commitment_state_management: Arc<StateManager<T>>,
}

impl <T: ManageState<Record=StateCommitmentPackage<AccountState>>> OptimisticHandler<T> {
    pub fn new(optimistic_commitment_state_management: Arc<StateManager<T>>) -> Self {
        OptimisticHandler { optimistic_commitment_state_management }
    }

    pub async fn get_pending_transaction_batch(&self, state_root: &str) -> Result<impl Reply> {
        let state_root_result = general_purpose::URL_SAFE.decode(state_root).expect("Error decoding state root.");
        let new_state_root_bytes: &[u8; 32] = <&[u8; 32]>::try_from(state_root_result.as_slice()).unwrap();
        let option = self.optimistic_commitment_state_management.get_state_record(new_state_root_bytes);
        match option {
            None => {
                Ok(json(&format!("No pending batches found for: {:?}", state_root_result)))
            }
            Some(pending_commitment) => {
                let ui_package: StateCommitmentPackageUI<AccountState> = (&pending_commitment).into();
                Ok(json(&ui_package))
            }
        }
    }

    pub async fn get_all_transactions(&self) -> Result<impl Reply> {
        let pending_commitments: Vec<([u8; 32], StateCommitmentPackage<AccountState>)> = self.optimistic_commitment_state_management.get_all_entries();
        let mut ui_pending_commitments = Vec::with_capacity(pending_commitments.iter().len());
        for (_, value) in pending_commitments {
            ui_pending_commitments.push(value.to_ui_package());
        }
        Ok(json(&ui_pending_commitments))
    }

}
