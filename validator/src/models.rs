use serde_derive::{Deserialize, Serialize};
use solana_sdk::signature::Signature;

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse {
    pub success: bool,
    pub signature: Signature
}