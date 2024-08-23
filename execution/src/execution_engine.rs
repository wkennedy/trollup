use std::str::FromStr;
use state::account_state::AccountState;
use state::block::Block;
use state::state_record;
use state::state_record::StateRecord;
use state::transaction::Transaction;
use state_commitment::state_commitment_layer::StateCommitmentLayer;
use state_management::state_management::{ManageState, StateManager};
use crate::transaction_pool::TransactionPool;

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
pub struct ExecutionEngine<'a, A: ManageState<Record=AccountState>, T: ManageState<Record=Transaction>, B: ManageState<Record=Block>> {
    account_state_management: &'a StateManager<A>,
    block_state_management: &'a StateManager<B>,
    transaction_pool: TransactionPool<'a, T>,
    account_state_commitment: StateCommitmentLayer<AccountState>,
    transaction_state_commitment: StateCommitmentLayer<Transaction>,
}

impl<'a, A: ManageState<Record=AccountState>, T: ManageState<Record=Transaction>, B: ManageState<Record=Block>> ExecutionEngine<'a, A, T, B> {
    pub fn new(account_state_management: &'a StateManager<A>, block_state_management: &'a StateManager<B>, transaction_pool: TransactionPool<'a, T>) -> Self {
        Self {
            account_state_management,
            block_state_management,
            transaction_pool,
            account_state_commitment: StateCommitmentLayer::<AccountState>::new(vec![]),
            transaction_state_commitment: StateCommitmentLayer::<Transaction>::new(vec![]),
        }
    }

    /// Starts the execution loop.
    ///
    /// This method runs an infinite loop until a break condition is met.
    /// It checks if the transaction pool size is greater than or equal to 4.
    /// If it is, it calls the `execute_block` method.
    /// If the pool size is less than 4, it breaks out of the loop.
    ///
    /// # Examples
    /// ```rust
    /// # use async_std::task;
    /// # struct TransactionPool {
    /// #     pub fn pool_size(&self) -> i32 { 0 }
    /// # }
    /// # struct Application {
    /// #     pub transaction_pool: TransactionPool,
    /// # }
    /// # impl Application {
    /// #     pub async fn execute_block(&mut self) {}
    /// #     pub async fn start(&mut self) {
    ///         loop {
    ///             if self.transaction_pool.pool_size() >= 4 {
    ///                 self.execute_block().await;
    ///             } else {
    ///                 break;
    ///             }
    ///         }
    ///     }
    /// # }
    ///
    /// # fn main() {
    /// #     let mut app = Application {
    /// #         transaction_pool: TransactionPool {},
    /// #     };
    /// #     let _ = task::block_on(app.start());
    /// # }
    /// ```
    pub async fn start(&mut self) {
        loop {
            if self.transaction_pool.pool_size() >= 4 {
                self.execute_block().await;
            } else {
                //TODO Add function to stop loop
                break;
            }
        }
    }

    /// Executes a block by processing a set of transactions.
    pub async fn execute_block(&mut self) {
        let transactions = &mut self.transaction_pool.get_next_transactions(4);
        let latest_block_id = self.account_state_management.get_latest_block().unwrap_or("BLOCK_0".to_string());
        let latest_block_number = u64::from_str(latest_block_id.split('_').last().unwrap_or("0")).unwrap_or(0) + 1;
        let mut accounts = Vec::new();
        let mut transaction_ids = Vec::with_capacity(transactions.len());
        let transactions_zk_gen = state_record::ZkProofSystem::<Transaction>::new(transactions.clone());
        let mut account_states = Vec::new();
        for tx in transactions {
            transaction_ids.push(tx.id);

            if !accounts.contains(&tx.account) {
                accounts.push(tx.account);
            }
            let account_state = self.execute_transaction(tx).expect("Failed processing transaction");
            account_states.push(account_state);
            self.transaction_state_commitment.update_record(tx.clone());
        }

        let account_zk_gen = state_record::ZkProofSystem::<AccountState>::new(account_states);
        let accounts_zk_proof = account_zk_gen.generate_proof();
        let tx_zk_proof = transactions_zk_gen.generate_proof();

        self.account_state_management.commit();
        let account_state_root = self.account_state_commitment.get_state_root().expect("Error getting account state root");
        let transaction_state_root = self.transaction_state_commitment.get_state_root().expect("Error getting transaction state root");

        let block = Block::new(latest_block_number, Box::from(transaction_state_root), tx_zk_proof.clone(),  Box::from(account_state_root), accounts_zk_proof.clone(), transaction_ids, accounts);
        println!("Saving latest block: {:?}", &block.get_key());
        self.block_state_management.set_latest_block(&block.get_key());
        self.block_state_management.set_state_record(&block.get_key(), block.clone());
        self.block_state_management.commit();
        self.account_state_commitment.commit_to_l1(&accounts_zk_proof).await;
    }

    // TODO: mock the Solana API
    /*
        pub fn load_and_execute_sanitized_transactions<CB: TransactionProcessingCallback>(
        &self,
        callbacks: &CB,
        sanitized_txs: &[impl SVMTransaction],
        check_results: Vec<TransactionCheckResult>,
        environment: &TransactionProcessingEnvironment,
        config: &TransactionProcessingConfig,
    ) -> LoadAndExecuteSanitizedTransactionsOutput
    */
    pub fn execute_transaction(&mut self, tx: &Transaction) -> Result<AccountState, String> {
        let mut account_state = self
            .account_state_management
            .get_state_record(&tx.account.as_ref()).unwrap_or_else(|| AccountState {
            address: tx.account,
            balance: 0,
            nonce: 0,
        });

        if account_state.nonce != tx.nonce {
            return Err("Invalid nonce".to_string());
        }

        account_state.nonce += 1;
        account_state.balance = tx.balance;

        println!("Account updated: {:?}", &account_state);
        self.account_state_management.set_state_record(&tx.account.as_ref(), account_state.clone());
        self.account_state_commitment.update_record(account_state.clone());

        Ok(account_state)
    }
}

