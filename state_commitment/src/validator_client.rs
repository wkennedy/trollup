use reqwest::Client;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use trollup_zk::prove::ProofPackagePrepared;
use base64::{Engine as _, engine::general_purpose};

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse {
    //TODO update this for the actual response from the validator
    pub message: String,
}

pub struct ValidatorClient {
    client: Client,
    base_url: String,
}

impl ValidatorClient {
    pub fn new(base_url: &str) -> Self {
        ValidatorClient {
            client: Client::new(),
            base_url: base_url.to_string(),
        }
    }

    pub async fn health_check(&self) -> Result<bool> {
        let response = self.client
            .get(&format!("{}/health", self.base_url))
            .send()
            .await?;

        Ok(response.status().is_success())
    }

    pub async fn prove(&self, proof_package: ProofPackagePrepared, new_state_root: &[u8; 32]) -> Result<ApiResponse> {
        let response = self.client
            .post(&format!("{}/prove/{}", self.base_url, general_purpose::STANDARD.encode(new_state_root)))
            .json(&proof_package)
            .send()
            .await?;

        if response.status().is_success() {
            let api_response: ApiResponse = response.json().await?;
            Ok(api_response)
        } else {
            Err(anyhow::anyhow!("API request failed: {:?}", response.status()))
        }
    }
}