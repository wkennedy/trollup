use serde_json::json;
use trollup_zk::prove::ProofPackagePrepared;
use trollup_zk::verify::verify_prepared_proof_package;
use warp::{http::StatusCode, Reply};
use warp::reply::json;
// use crate::config::Config;

use crate::Result;

// lazy_static! {
//     static ref CONFIG: Config = Config::build().unwrap();
// }

#[utoipa::path(
    post,
    path = "/prove",
    tag = "",
    responses(
(status = 200, description = "Submit proof for verification")),
)]
pub async fn prove(proof_package_prepared: ProofPackagePrepared) -> Result<impl Reply> {
    let is_valid = verify_prepared_proof_package(&proof_package_prepared);
    println!("result {:?}", &is_valid);

    Ok(json(&""))
}

pub async fn health_handler() -> Result<impl Reply> {
    Ok(StatusCode::OK)
}
