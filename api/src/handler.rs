use execution::transaction_pool::TransactionPool;
use lazy_static::lazy_static;
use solana_sdk::transaction::Transaction;
use state::transaction::convert_to_trollup_transaction;
use std::sync::{Arc};
use tokio::sync::Mutex;
use warp::{http::StatusCode, reply::json, Filter, Rejection, Reply};
use state::config::TrollupConfig;

type Result<T> = std::result::Result<T, Rejection>;

lazy_static! {
    static ref CONFIG: TrollupConfig = TrollupConfig::build().unwrap();
}

pub struct Handler {
    transaction_pool: Arc<Mutex<TransactionPool>>,
}

impl Handler {
    pub fn new(transaction_pool: Arc<Mutex<TransactionPool>>) -> Self {
        Handler { transaction_pool }
    }

    pub async fn send_transaction_handler(&self, transaction: Transaction) -> Result<impl Reply> {
        let mut pool = self.transaction_pool.lock().await;
        let trollup_transaction = convert_to_trollup_transaction(transaction).unwrap();
        pool.add_transaction(trollup_transaction);
        Ok(json(&"Transaction submitted successfully"))
    }

    pub async fn send_transaction_optimistic_handler(&self, transaction: Transaction) -> Result<impl Reply> {
        let mut pool = self.transaction_pool.lock().await;
        let mut trollup_transaction = convert_to_trollup_transaction(transaction).unwrap();
        trollup_transaction.optimistic = true;
        pool.add_transaction(trollup_transaction);
        Ok(json(&"Optimistic transaction submitted successfully"))
    }

    pub async fn health_handler(&self) -> Result<impl Reply> {
        Ok(StatusCode::OK)
    }
}

// Function to create filter with Handler
pub fn with_handler(
    transaction_pool: Arc<Mutex<TransactionPool>>,
) -> impl Filter<Extract=(Handler,), Error=std::convert::Infallible> + Clone {
    warp::any().map(move || Handler::new(Arc::clone(&transaction_pool)))
}