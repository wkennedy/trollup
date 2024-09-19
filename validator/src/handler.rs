use serde_json::json;
use trollup_zk::prove::ProofPackagePrepared;
use trollup_zk::verify::verify_prepared_proof_package;
use warp::{http::StatusCode, Rejection, Reply};
use warp::reply::json;
use crate::commitment::verify_and_commit;
// use crate::config::Config;


// lazy_static! {
//     static ref CONFIG: Config = Config::build().unwrap();
// }

type Result<T> = std::result::Result<T, Rejection>;

#[utoipa::path(
    post,
    path = "/prove/{new_state_root}",
    request_body = ProofPackagePrepared,
    params(
        ("new_state_root" = i64, Path, description = "The new state root for the transaction batch")
    ),
    tag = "",
    responses(
        (status = 200, description = "Result of proof verification")
    ),
)]
pub async fn prove(proof_package_prepared: ProofPackagePrepared, new_state_root: String) -> Result<impl Reply> {
    //todo validate input
    let new_state_root_bytes: &[u8; 32] = <&[u8; 32]>::try_from(new_state_root.as_bytes()).unwrap();
    let result = verify_and_commit(proof_package_prepared, new_state_root_bytes.clone()).await;
    match result {
        Ok(is_valid) => {
            println!("result {:?}", &is_valid);
            Ok(json(&""))
        }
        Err(error) => {
            println!("result {:?}", &error);
            Ok(json(&""))
        }
    }
}

pub async fn health_handler() -> Result<impl Reply> {
    Ok(StatusCode::OK)
}
