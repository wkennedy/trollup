use std::marker::PhantomData;
use borsh::{from_slice, to_vec};
use sled::{Config, Db};
use state::state_record::StateRecord;
use crate::state_management::ManageState;

/// Struct for managing state using Sled as the underlying database.
///
/// # Generic Parameters
///
/// - `S`: The state record type which should implement the `StateRecord` trait.
///
/// # Fields
///
/// - `db`: The instance of Sled database.
/// - `_marker`: A marker field used to specify the type of state record stored in the database.
#[derive(Debug, Clone)]
pub struct SledStateManagement<S: StateRecord> {
    db: Db,
    _marker: PhantomData<S>,
}

impl<S: StateRecord> ManageState for SledStateManagement<S> {
    type Record = S;

    #[allow(unused_variables)]
    fn new(path: &str) -> Self {
        let config = Config::new().temporary(true);

        // TODO get path from config
        let db = config.open().expect("");
        // let db = sled::open(path).expect("Failed to open database");
        Self { db, _marker: PhantomData }
    }

    fn get_all_entries(&self) -> Vec<([u8;32], S)> {
        self.db
            .iter()
            .filter_map(|result| {
                result.ok().and_then(|(key, value)| {
                    // Try to convert the key to a [u8; 32]
                    let key_array: Result<[u8; 32], _> = key.as_ref().try_into();

                    // If the key conversion succeeds and we can deserialize the value,
                    // include this entry in the result
                    if let (Ok(key_32), Ok(deserialized_value)) = (key_array, S::try_from_slice(&value)) {
                        Some((key_32, deserialized_value))
                    } else {
                        None
                    }
                })
            })
            .collect()
    }

    fn get_state_record(&self, key: &[u8]) -> Option<S> {
        self.db
            .get(key)
            .ok()
            .flatten()
            .and_then(|ivec| from_slice::<S>(&ivec).ok())
    }

    fn set_state_record(&self, state: &S) {
        let serialized = to_vec(&state).expect("Failed to serialize account state");
        self.db.insert(state.get_key(), serialized).expect("Failed to insert account state");
    }

    fn set_state_records(&self, states: &Vec<Self::Record>) {
        let mut batch = sled::Batch::default();
        for state in states {
            let serialized = to_vec(&state).expect("Failed to serialize account state");
            batch.insert(&state.get_key(), serialized);
        }
        self.db.apply_batch(batch).expect("Failed to insert account state");
    }

    fn set_latest_block_id(&self, value: &[u8; 32]) {
        self.db.insert("LATEST_BLOCK", value).expect("Failed to insert LATEST_BLOCK key");
    }

    fn get_latest_block_id(&self) -> Option<[u8; 32]> {
        self.db
            .get("LATEST_BLOCK")
            .ok()
            .flatten()
            .and_then(|ivec| from_slice::<[u8; 32]>(&ivec).ok())
    }

    fn commit(&self) {
        self.db.flush().expect("Failed to commit database");
    }
}
