use crate::config::{Config, TrollupConfig};
use lazy_static::lazy_static;
use state::block::Block;
use state_management::state_management::{ManageState, StateManager};
use std::sync::Arc;
use warp::{reply::json, Rejection, Reply};

type Result<T> = std::result::Result<T, Rejection>;

lazy_static! {
    static ref CONFIG: TrollupConfig = TrollupConfig::build().unwrap();
}

pub struct BlockHandler<B: ManageState<Record=Block>> {
    block_state_management: Arc<StateManager<B>>,
}

impl <B: ManageState<Record=Block>> BlockHandler<B> {
    pub fn new(block_state_management: Arc<StateManager<B>>) -> Self {
        BlockHandler { block_state_management }
    }

    pub async fn get_block(&self, block_id: u64) -> Result<impl Reply> {
        let id = Block::get_id(block_id);
        let option = self.block_state_management.get_state_record(&id);
        match option {
            None => {
                Ok(json(&format!("No block found for: {:?}", block_id)))
            }
            Some(block) => {
                Ok(json(&format!("block details: {:?}", block)))
            }
        }
    }

    pub async fn get_latest_block(&self) -> Result<impl Reply> {
        let option = self.block_state_management.get_latest_block_id();
        match option {
            None => {
                Ok(json(&"No blocks exist".to_string()))
            }
            Some(block) => {
                let block_option = self.block_state_management.get_state_record(&block);
                Ok(json(&format!("Block details: {:?}", block_option)))
            }
        }
    }

    pub async fn get_all_blocks(&self) -> Result<impl Reply> {
        let blocks: Vec<([u8;32], Block)> = self.block_state_management.get_all_entries();
        Ok(json(&blocks))
    }
}