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
pub struct SledStateManagement<S: StateRecord> {
    db: Db,
    _marker: PhantomData<S>,
}

impl<S: StateRecord> ManageState for SledStateManagement<S> {
    type Record = S;

    #[allow(unused_variables)]
    fn new(path: &str) -> Self {
        let config = Config::new().temporary(true);

        let db = config.open().expect("");
        // let db = sled::open(path).expect("Failed to open database");
        Self { db, _marker: PhantomData }
    }

    fn get_state_record(&self, key: &[u8]) -> Option<S> {
        self.db
            .get(key)
            .ok()
            .flatten()
            .and_then(|ivec| from_slice::<S>(&ivec).ok())
    }

    fn set_state_record(&self, key: &[u8], state: S) {
        let serialized = to_vec(&state).expect("Failed to serialize account state");
        self.db.insert(key, serialized).expect("Failed to insert account state");
    }

    fn set_latest_block(&self, value: Vec<u8>) {
        self.db.insert("LATEST_BLOCK", value).expect("Failed to insert LATEST_BLOCK key");
    }

    fn get_latest_block(&self) -> Option<String> {
        self.db
            .get("LATEST_BLOCK")
            .ok()
            .flatten()
            .and_then(|ivec| from_slice::<String>(&ivec).ok())
    }

    fn commit(&self) {
        self.db.flush().expect("Failed to commit database");
    }
}
