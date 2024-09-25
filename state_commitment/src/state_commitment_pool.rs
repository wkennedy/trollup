use crate::state_commitment_layer::StateCommitmentPackage;
use state::state_record::StateRecord;
use state_management::state_management::ManageState;
use std::collections::VecDeque;

pub trait StatePool {
    type Record: StateRecord;
    fn new() -> Self;
    fn add(&mut self, package: StateCommitmentPackage<Self::Record>);
    fn get_next(&mut self) -> Option<StateCommitmentPackage<Self::Record>>;
    fn pool_size(&self) -> usize;
    fn get_next_chunk(&mut self, chunk: u32) -> Vec<StateCommitmentPackage<Self::Record>>;
}

pub struct StateCommitmentPool<S: StateRecord> {
    pool: VecDeque<StateCommitmentPackage<S>>,
}
impl <S: StateRecord> StatePool for StateCommitmentPool<S> {
    type Record = S;

    fn new() -> Self {
        Self {
            pool: VecDeque::new()
        }
    }

    fn add(&mut self, package: StateCommitmentPackage<Self::Record>) {
        self.pool.push_back(package);
    }

    fn get_next(&mut self) -> Option<StateCommitmentPackage<S>> {
        self.pool.pop_front()
    }

    fn pool_size(&self) -> usize {
        self.pool.len()
    }

    fn get_next_chunk(&mut self, chunk: u32) -> Vec<StateCommitmentPackage<S>> {
        let mut packages = Vec::new();
        if self.pool_size() == 0 {
            return vec![]
        }

        let to = chunk.min(self.pool_size() as u32);
        for _ in 0..to {
            if let Some(package) = self.pool.pop_front() {
                packages.push(package);
            } else {
                break;
            }
        }
        packages
    }
}