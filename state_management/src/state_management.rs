use state::state_record::StateRecord;

/// `ManageState` is a trait that provides methods for managing state records and the latest block value.
pub trait ManageState {
    type Record: StateRecord;

    fn new(path: &str) -> Self;
    fn get_state_record(&self, key: &[u8]) -> Option<Self::Record>;
    fn set_state_record(&self, key: &[u8], state: Self::Record);
    fn set_latest_block(&self, value: Vec<u8>);
    fn get_latest_block(&self) -> Option<String>;
    fn commit(&self);
}

/// A generic struct used to manage the state of any type that implements the `ManageState` trait.
///
/// # Example
///
/// ```
/// use my_library::StateManager;
///
/// struct MyState {
///     // implementation details
/// }
///
/// impl ManageState for MyState {
///     // implementation details
/// }
///
/// let state_manager = StateManager {
///     manage_state: MyState,
/// };
/// ```
///
/// # Generic Parameters
///
/// - `T`: The type that implements the `ManageState` trait for state management.
pub struct StateManager<T: ManageState> {
    pub manage_state: T,
}

impl<T: ManageState> StateManager<T> {
    pub fn new(path: &str) -> Self {
        Self {
            manage_state: T::new(path),
        }
    }

    pub fn get_state_record(&self, key: &[u8]) -> Option<T::Record> {
        self.manage_state.get_state_record(key)
    }

    pub fn get_latest_block(&self) -> Option<String> {
        self.manage_state.get_latest_block()
    }

    pub fn set_latest_block(&self, key: &[u8]) {
        self.manage_state.set_latest_block(key.to_vec());
    }

    pub fn set_state_record(&self, key: &[u8], state: T::Record) {
        self.manage_state.set_state_record(key, state)
    }

    pub fn commit(&self) {
        self.manage_state.commit()
    }
}