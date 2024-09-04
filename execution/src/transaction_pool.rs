use std::collections::VecDeque;
use state::state_record::StateRecord;
use state::transaction::Transaction;
use state_management::state_management::{ManageState, StateManager};

/// TransactionPool is a struct that represents a pool of transactions.
///
/// The transactions are stored in a VecDeque, which allows for efficient insertion and removal
/// of transactions from both ends of the queue.
///
/// A TransactionPool requires a StateManager to manage the state of the transactions.
///
/// # Generic Parameters
/// - `'a`: The lifetime parameter for the StateManager reference.
/// - `M`: The StateManager's type implementing the ManageState trait,
///        with the associated type `Record` being a Transaction.
///
/// # Fields
/// - `pool`: A VecDeque that stores the transactions.
/// - `state_management`: A reference to the StateManager that manages the state of the transactions.
#[derive(Debug, Clone)]
pub struct TransactionPool<'a, M: ManageState<Record = Transaction>> {
    pool: VecDeque<Transaction>,
    state_management: &'a StateManager<M>,
}

impl<'a, M: ManageState<Record = Transaction>> TransactionPool<'a, M> {
    pub fn new(state_management: &'a StateManager<M>) -> Self {
        Self {
            state_management,
            pool: VecDeque::new()
        }
    }

    pub fn add_transaction(&mut self, tx: Transaction) {
        match tx.get_key() {
            None => {}
            Some(key) => {
                self.state_management.set_state_record(&key, tx.clone());
                self.pool.push_back(tx);}
        }

    }

    pub fn get_next_transaction(&mut self) -> Option<Transaction> {
        self.pool.pop_front()
    }

    pub fn pool_size(&self) -> usize {
        self.pool.len()
    }

    pub fn get_next_transactions(&mut self, chunk: u32) -> Vec<Transaction> {
        let mut transactions = Vec::new();
        if self.pool_size() == 0 {
            return vec![]
        }

        let to = chunk.min(self.pool_size() as u32);
        for _ in 0..to {
            if let Some(transaction) = self.pool.pop_front() {
                transactions.push(transaction);
            } else {
                break;
            }
        }
        transactions
    }
}