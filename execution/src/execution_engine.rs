use crate::processor::{create_transaction_batch_processor, get_transaction_check_results};
use crate::transaction_pool::TransactionPool;
use solana_compute_budget::compute_budget::ComputeBudget;
use solana_sdk::account::ReadableAccount;
use solana_sdk::feature_set::FeatureSet;
use solana_sdk::fee::FeeStructure;
use solana_sdk::hash::Hash;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::rent_collector::RentCollector;
use solana_sdk::transaction::SanitizedTransaction;
use solana_svm::account_loader::{LoadedTransaction, TransactionLoadResult};
use solana_svm::transaction_processor::{LoadAndExecuteSanitizedTransactionsOutput, TransactionProcessingConfig, TransactionProcessingEnvironment};
use solana_svm::transaction_results::TransactionExecutionResult;
use state::account_state::AccountState;
use state::state_record::StateRecord;
use state::transaction::TrollupTransaction;
use state_commitment::state_commitment_pool::{StateCommitmentPool, StatePool};
use state_management::account_loader::TrollupAccountLoader;
use state_management::state_management::{ManageState, StateManager};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use solana_program_runtime::log_collector::log::info;
use state_commitment::state_commitment_layer::StateCommitmentPackage;

#[derive(PartialEq, Eq, Debug)]
enum EngineState {
    Running,
    Stopped,
    Initialized,
}

/// This struct represents an execution engine for managing the state of accounts, transactions, and blocks.
///
/// # Generic Parameters
/// - `'a`: lifetime parameter representing the lifetime of the execution engine instance.
/// - `A`: a type that implements the `ManageState` trait with `Record` associated type set to `AccountState`. This type is used for managing the state of accounts.
/// - `T`: a type that implements the `ManageState` trait with `Record` associated type set to `Transaction`. This type is used for managing the state of transactions.
/// - `B`: a type that implements the `ManageState` trait with `Record` associated type set to `Block`. This type is used for managing the state of blocks.
///
/// # Fields
/// - `account_state_management`: a reference to a `StateManager` instance for managing the state of accounts.
/// - `block_state_management`: a reference to a `StateManager` instance for managing the state of blocks.
/// - `transaction_pool`: a `TransactionPool` instance for managing the pool of unprocessed transactions.
/// - `account_state_commitment`: a `StateCommitmentLayer` instance for committing the state changes of accounts.
/// - `transaction_state_commitment`: a `StateCommitmentLayer` instance for committing the state changes of transactions.
pub struct ExecutionEngine<'a, A: ManageState<Record=AccountState>> {
    account_state_management: &'a StateManager<A>,
    transaction_pool: Arc<Mutex<TransactionPool>>,
    commitment_pool: Arc<Mutex<StateCommitmentPool<AccountState>>>,
    engine_state: EngineState,
}

impl<'a, A: ManageState<Record=AccountState>> ExecutionEngine<'a, A> {
    /// Creates a new instance of the `ExecutionEngine`.
    ///
    /// # Parameters
    /// - `account_state_management`: A reference to a `StateManager` instance for managing the state of accounts.
    /// - `transaction_pool`: An atomic reference counter to a thread-safe `TransactionPool` instance for managing the pool of unprocessed transactions.
    /// - `commitment_pool`: An atomic reference counter to a thread-safe `StateCommitmentPool` instance for committing the state changes of accounts.
    ///
    /// # Returns
    /// A new `ExecutionEngine` instance initialized with the provided `StateManager`, `TransactionPool`, and `StateCommitmentPool`.
    pub fn new(account_state_management: &'a StateManager<A>, transaction_pool: Arc<Mutex<TransactionPool>>, commitment_pool: Arc<Mutex<StateCommitmentPool<AccountState>>>) -> Self {
        Self {
            account_state_management,
            transaction_pool,
            commitment_pool,
            engine_state: EngineState::Initialized,
        }
    }

    /// Starts the execution loop.
    ///
    /// This method runs an infinite loop until a break condition is met.
    /// It checks if the transaction pool size is greater than or equal to 4.
    /// If it is, it calls the `execute_block` method.
    /// If the pool size is less than 4, it breaks out of the loop.
    pub async fn start(&mut self) {
        self.engine_state = EngineState::Running;
        info!("Execution Engine started.");
        loop {
            if self.engine_state == EngineState::Stopped {
                info!("Execution Engine stopped.");
                break;
            } else {
                self.execute_block().await;
            }
        }
    }

    pub async fn stop(&mut self) {
        info!("Stopping Execution Engine");
        self.engine_state = EngineState::Stopped;
    }

    /// Executes a block by processing a set of transactions.
    pub async fn execute_block(&mut self) {
        let mut tx_pool = self.transaction_pool.lock().unwrap();
        let transactions = tx_pool.get_next_transactions(4);
        drop(tx_pool);
        if transactions.is_empty() {
            return;
        }

        let sanitized_txs = batch_sanitize_transactions(&transactions);

        // Create a mapping of signatures to transactions
        let tx_map: HashMap<[u8; 32], &TrollupTransaction> = transactions
            .iter()
            .map(|tx| (tx.get_key(), tx))
            .collect();

        let results = self.execute_svm_transactions(sanitized_txs.clone());
        let loaded_txs = results.loaded_transactions;

        let exec_results = results.execution_results;

        let successful_outcomes = extract_successful_transactions(&tx_map, &loaded_txs, &exec_results);

        let mut successful_txs: Vec<TrollupTransaction> = Vec::new();
        let mut transaction_ids = Vec::with_capacity(successful_outcomes.len());
        let mut account_states: Vec<AccountState> = Vec::new();
        for outcome in successful_outcomes {
            transaction_ids.push(outcome.trollup_transaction.get_key());
            account_states.extend(outcome.accounts);
            successful_txs.push(outcome.trollup_transaction);
        }

        let commitment_package = StateCommitmentPackage {
            state_records: account_states,
            transactions: successful_txs,
            transaction_ids,
        };

        let mut commit_pool = self.commitment_pool.lock().unwrap();
        commit_pool.add(commitment_package);
    }


    pub fn execute_svm_transactions(&self, transactions: Vec<SanitizedTransaction>) -> LoadAndExecuteSanitizedTransactionsOutput {
        let compute_budget = ComputeBudget::default();
        let feature_set = FeatureSet::all_enabled();
        let fee_structure = FeeStructure::default();
        let lamports_per_signature = fee_structure.lamports_per_signature;
        let rent_collector = RentCollector::default();

        let account_loader = TrollupAccountLoader::new(self.account_state_management);

        let (processor, _fork_graph) =
            create_transaction_batch_processor(&account_loader, &feature_set, &compute_budget);

        let processing_environment = TransactionProcessingEnvironment {
            blockhash: Hash::default(),
            epoch_total_stake: None,
            epoch_vote_accounts: None,
            feature_set: Arc::new(feature_set),
            fee_structure: Some(&fee_structure),
            lamports_per_signature,
            rent_collector: Some(&rent_collector),
        };

        let processing_config = TransactionProcessingConfig {
            compute_budget: Some(compute_budget),
            ..Default::default()
        };

        let results = processor.load_and_execute_sanitized_transactions(
            &account_loader,
            &transactions,
            get_transaction_check_results(transactions.len(), lamports_per_signature),
            &processing_environment,
            &processing_config,
        );

        results
    }
}

pub fn batch_sanitize_transactions(transactions: &Vec<TrollupTransaction>) -> Vec<SanitizedTransaction> {
    transactions
        .into_iter()
        .filter_map(|tx| {
            state::transaction::convert_to_sanitized_transaction(tx)
                .map_err(|e| {
                    eprintln!("Failed to sanitize transaction: {:?}", e);
                    e
                })
                .ok()
        })
        .collect()
}

struct ExecutionOutcome {
    trollup_transaction: TrollupTransaction,
    accounts: Vec<AccountState>,
}

fn extract_successful_transactions(
    tx_map: &HashMap<[u8; 32], &TrollupTransaction>,
    loaded_txs: &[TransactionLoadResult],
    exec_results: &[TransactionExecutionResult],
) -> Vec<ExecutionOutcome> {
    let mut execution_outcomes = Vec::new();
    for (i, (_key, value)) in tx_map.iter().enumerate() {
        let loaded_tx = &loaded_txs[i].clone().unwrap();
        let x1 = &exec_results[i];
        match x1 {
            TransactionExecutionResult::Executed { .. } => {
                execution_outcomes.push(ExecutionOutcome {
                    trollup_transaction: value.clone().clone(),
                    accounts: extract_accounts(&loaded_tx.clone()),
                });
            }
            TransactionExecutionResult::NotExecuted(_) => {}
        }
    };
    execution_outcomes
}

fn extract_accounts(loaded_tx: &LoadedTransaction) -> Vec<AccountState> {
    loaded_tx.accounts
        .iter()
        .map(|account| {
            AccountState {
                address: Pubkey::from(account.0.to_bytes()),
                lamports: account.1.lamports(),
                data: account.1.data().to_vec(),
                owner: *account.1.owner(),
                executable: account.1.executable(),
                rent_epoch: account.1.rent_epoch(),
            }
        })
        .collect()
}


