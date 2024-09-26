use crate::commitment::verify_and_commit;
use base64::{engine::general_purpose, Engine as _};
use lazy_static::lazy_static;
use log::info;
use trollup_zk::prove::ProofPackagePrepared;
use warp::reply::json;
use warp::{http::StatusCode, Rejection, Reply};
use crate::config::Config;

lazy_static! {
    static ref CONFIG: Config = Config::build().unwrap();
}

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
    let state_root_result = general_purpose::STANDARD.decode(new_state_root);
    match state_root_result {
        Ok(state_root) => {
            let new_state_root_bytes: &[u8; 32] = <&[u8; 32]>::try_from(state_root.as_slice()).unwrap();
            let result = verify_and_commit(proof_package_prepared, new_state_root_bytes.clone()).await;
            match result {
                // TODO finalize results response
                Ok(is_valid) => {
                    info!("result {:?}", &is_valid);
                    Ok(json(&""))
                }
                Err(error) => {
                    info!("result {:?}", &error);
                    Ok(json(&""))
                }
            }
        }

        Err(error) => {
            info!("result {:?}", &error);
            Ok(json(&""))
        }
    }

}

pub async fn health_handler() -> Result<impl Reply> {
    Ok(StatusCode::OK)
}
