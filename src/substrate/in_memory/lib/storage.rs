use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Clone, Default)]
pub struct InMemoryStorage {
    assets: Arc<Mutex<HashMap<String, String>>>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self::default()
    }

    pub(super) fn assets(&self) -> Arc<Mutex<HashMap<String, String>>> {
        Arc::clone(&self.assets)
    }
}
