use std::cell::RefCell;
use std::future::Future;
use std::sync::OnceLock;

use tokio::sync::mpsc;

use crate::store::message::{StoreMessage, StoreRequest, StoreResponse};
use crate::store::state::Store;
use crate::store_error::StoreError;
use crate::substrate::schema_registry::SchemaBackedSubstrate;

static GLOBAL_SENDER: OnceLock<mpsc::Sender<StoreMessage>> = OnceLock::new();

thread_local! {
    static OVERRIDE_SENDER: RefCell<Option<mpsc::Sender<StoreMessage>>> = RefCell::new(None);
}

pub struct EntityServer;

struct OverrideGuard {
    previous: Option<mpsc::Sender<StoreMessage>>,
}

impl Drop for OverrideGuard {
    fn drop(&mut self) {
        OVERRIDE_SENDER.with(|override_sender| {
            *override_sender.borrow_mut() = self.previous.take();
        });
    }
}

impl EntityServer {
    pub fn init<S>(substrate: S)
    where
        S: SchemaBackedSubstrate,
    {
        let (tx, rx) = mpsc::channel(32);
        let store = Store::new(substrate);
        tokio::spawn(async move { store.run(rx).await });
        GLOBAL_SENDER.set(tx).expect("EntityServer already initialized");
    }

    fn sender() -> mpsc::Sender<StoreMessage> {
        OVERRIDE_SENDER
            .with(|o| o.borrow().clone())
            .unwrap_or_else(|| GLOBAL_SENDER.get().expect("EntityServer not initialized").clone())
    }

    pub(crate) async fn request(request: StoreRequest) -> Result<StoreResponse, StoreError> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        Self::sender()
            .send(StoreMessage::Request { request, reply: tx })
            .await
            .map_err(|_| StoreError::Unavailable)?;
        rx.await.map_err(|_| StoreError::Unavailable)?
    }

    pub async fn with<S, F, Fut>(substrate: S, f: F)
    where
        S: SchemaBackedSubstrate,
        F: FnOnce() -> Fut,
        Fut: Future<Output = ()>,
    {
        let (tx, rx) = mpsc::channel(32);
        tokio::spawn(Store::new(substrate).run(rx));
        let previous = OVERRIDE_SENDER.with(|override_sender| override_sender.borrow_mut().replace(tx));
        let _guard = OverrideGuard { previous };
        f().await;
    }
}
