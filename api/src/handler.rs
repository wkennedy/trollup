use lazy_static::lazy_static;
use solana_sdk::transaction::Transaction;
use warp::{http::StatusCode, reply::json, Filter, Rejection, Reply};
use warp::reply::Json;
use state::transaction::{convert_to_trollup_transaction, TrollupTransaction};
use std::sync::{Arc, Mutex};
use execution::transaction_pool::TransactionPool;

type Result<T> = std::result::Result<T, Rejection>;

pub struct Handler {
    transaction_pool: Arc<Mutex<TransactionPool>>,
}

impl Handler {
    pub fn new(transaction_pool: Arc<Mutex<TransactionPool>>) -> Self {
        Handler { transaction_pool }
    }


    pub async fn get_transaction_handler(&self, signature: String) -> Result<impl Reply> {
        // Implement logic to get transaction from pool or storage
        // For now, we'll just return a placeholder response
        Ok(json(&format!("Transaction details for signature: {}", signature)))
    }


    pub async fn send_transaction_handler(&self, transaction: Transaction) -> Result<impl Reply> {
        let mut pool = self.transaction_pool.lock().unwrap();
        // Convert Solana Transaction to TrollupTransaction (you'll need to implement this conversion)
        let trollup_transaction = convert_to_trollup_transaction(transaction).unwrap();
        pool.add_transaction(trollup_transaction);
        Ok(json(&"Transaction submitted successfully"))
    }

    pub async fn health_handler(&self) -> Result<impl Reply> {
        Ok(StatusCode::OK)
    }
}

// Function to create filter with Handler
pub fn with_handler(
    transaction_pool: Arc<Mutex<TransactionPool>>,
) -> impl Filter<Extract = (Handler,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || Handler::new(Arc::clone(&transaction_pool)))
}