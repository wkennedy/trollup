use std::collections::HashSet;
use std::str::FromStr;
use lazy_static::lazy_static;
use solana_client::rpc_client::RpcClient;
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
use log::{debug, info};
use state::account_state::AccountState;
use state::config::TrollupConfig;
use crate::state_management::{ManageState, StateManager};

lazy_static! {
    static ref CONFIG: TrollupConfig = TrollupConfig::build().unwrap();
}

pub struct TrollupAccountLoader<'a, A: ManageState> {
    cache: RwLock<HashMap<[u8; 32], AccountSharedData>>,
    account_state_management: &'a StateManager<A>,
    rpc_client: RpcClient,
    program_ids: HashSet<Pubkey>
}

impl<'a, A: ManageState<Record=AccountState>> TrollupAccountLoader<'a, A> {
    pub fn new(account_state_management: &'a StateManager<A>) -> Self {
        let mut program_ids = HashSet::new();
        // Add the Token program ID
        info!("{:?}", &CONFIG.program_ids_to_load);
        for program_id in &CONFIG.program_ids_to_load {
            program_ids.insert(Pubkey::from_str(program_id).expect("Error getting pubkey from program ID str"));
        }
        // let _ = CONFIG.program_ids_to_load.iter().map(|program_id| {info!("PROGRAM_IDS: {:?}", &program_id);program_ids.insert(Pubkey::from_str(&program_id).unwrap())});
        info!("PROGRAM_IDS: {:?}", &program_ids);
        // program_ids.insert(Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap());
        // program_ids.insert(Pubkey::from_str("1111111QLbz7JHiBTspS962RLKV8GndWFwiEaqKM").unwrap());
        // program_ids.insert(Pubkey::from_str("11111111111111111111111111111111").unwrap());

        Self {
            cache: RwLock::new(HashMap::new()),
            account_state_management,
            rpc_client: RpcClient::new_with_commitment(&CONFIG.rpc_urls.get("Dev").unwrap(), CommitmentConfig::confirmed()), //TODO load from config
            program_ids,
        }
    }
}

impl<'a, A: ManageState<Record=AccountState>> TransactionProcessingCallback for TrollupAccountLoader<'a, A> {
    fn account_matches_owners(&self, account: &Pubkey, owners: &[Pubkey]) -> Option<usize> {
        self.get_account_shared_data(account)
            .and_then(|account| owners.iter().position(|key| account.owner().eq(key)))
    }

    fn get_account_shared_data(&self, pubkey: &Pubkey) -> Option<AccountSharedData> {
        info!("Getting shared account for {:?}", pubkey);

        // Check cache first
        if let Some(account) = self.cache.read().unwrap().get(&pubkey.to_bytes()) {
            info!("Found in cache... shared account for {:?}", pubkey);
            return Some(account.clone());
        }

        // If not in cache, try to load from state management
        if let Some(account) = self.account_state_management.get_state_record(&pubkey.to_bytes()) {
            info!("Found in state management... shared account for {:?}", pubkey);
            let account_shared_data: AccountSharedData = account.into();
            self.cache.write().unwrap().insert(pubkey.to_bytes(), account_shared_data.clone());
            return Some(account_shared_data);
        }
        
        if self.program_ids.contains(pubkey) {
            let account_data = self.rpc_client.get_account_with_commitment(pubkey, CommitmentConfig::confirmed()).ok()?;
            if let Some(account_data) = account_data.value {
                let account_shared_data = AccountSharedData::from(account_data);
                self.cache.write().unwrap().insert(pubkey.to_bytes(), account_shared_data.clone());
                return Some(account_shared_data);
            }
        }

        // If not found in state management, create a default account
        info!("Not found... creating default account for {:?}", pubkey);
        // TODO for now all new accounts are owned by the System program, this will need to change
        let default_account = AccountSharedData::new(
            10000000000000,
            0,
            &Pubkey::from_str("11111111111111111111111111111111").unwrap()
        );
        self.cache.write().unwrap().insert(pubkey.to_bytes(), default_account.clone());
        Some(default_account)
    }

    fn add_builtin_account(&self, name: &str, program_id: &Pubkey) {
        let account_data = native_loader::create_loadable_account_with_fields(name, (5000, 0));
        self.cache
            .write()
            .unwrap()
            .insert(program_id.to_bytes(), account_data);
    }
}