use lazy_static::lazy_static;
use warp::{http::StatusCode, reply::json, Reply};
use warp::reply::Json;
// use crate::config::Config;

use crate::Result;

// lazy_static! {
//     static ref CONFIG: Config = Config::build().unwrap();
// }

#[utoipa::path(
    get,
    path = "/get-transaction/{signature}",
    tag = "",
    responses(
    (status = 200, description = "Transaction data retrieval successful"),
    (status = 404, description = "Transaction not found."),
    ),
    params(
    ("signature" = String, Path, description = "The signature of the transaction"),
    )
)]
pub async fn get_transaction_handler(signature: String) -> Result<impl Reply> {
        //signature [u8; 32]
    // println!("result {:?}", result);
    Ok(json(&""))
}

#[utoipa::path(
    post,
    path = "/send-transaction",
    tag = "",
    responses(
(status = 200, description = "Transaction submitted successful")),
)]
pub async fn send_transaction_handler(transaction: TrollupTransaction) -> Result<impl Reply> {
    // println!("result {:?}", &result);
    Ok(json(&""))
}

pub async fn health_handler() -> Result<impl Reply> {
    Ok(StatusCode::OK)
}
