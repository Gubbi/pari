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

    /// Replace an asset's stored content directly, bypassing the
    /// codec/executor write path. Used by tests that need to seed the
    /// substrate with content the standard write path can't produce —
    /// e.g. a sparse slice whose decode response exercises the
    /// store's missing-field handling.
    pub fn put(&self, path: impl Into<String>, content: impl Into<String>) {
        self.assets
            .lock()
            .unwrap()
            .insert(path.into(), content.into());
    }
}
