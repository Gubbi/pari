# store-server

**Store Layer → `store_layer/entity-server/`**

---

## Purpose

`EntityServer` is the Store Layer's singleton actor — it owns the `Store<S>` and processes all mutations and queries through an async message channel. No store state is ever exposed directly; all access is mediated by the message protocol.

---

## EntityServer — Infrastructure

```rust
static GLOBAL_SENDER: OnceLock<mpsc::Sender<StoreMessage>> = OnceLock::new();

thread_local! {
    static OVERRIDE_SENDER: RefCell<Option<mpsc::Sender<StoreMessage>>> = RefCell::new(None);
}

pub struct EntityServer;

impl EntityServer {
    pub fn init(substrate: impl Substrate + Send + 'static) {
        let (tx, rx) = mpsc::channel(32);
        let store = Store::new(substrate);
        async_runtime::spawn(store.run(rx));
        GLOBAL_SENDER.set(tx).expect("EntityServer already initialized");
    }

    pub fn sender() -> mpsc::Sender<StoreMessage> {
        OVERRIDE_SENDER.with(|o| o.borrow().clone())
            .unwrap_or_else(|| GLOBAL_SENDER.get().expect("EntityServer not initialized").clone())
    }

    /// Used in tests — scoped override for the calling thread.
    pub fn with(substrate: impl Substrate + Send + 'static, f: impl FnOnce()) {
        let (tx, rx) = mpsc::channel(32);
        async_runtime::spawn(Store::new(substrate).run(rx));
        OVERRIDE_SENDER.with(|o| *o.borrow_mut() = Some(tx));
        f();
        OVERRIDE_SENDER.with(|o| *o.borrow_mut() = None);
    }
}
```

---

## Message Protocol

Internal to this module — not exposed outside.

```rust
enum StoreRequest {
    Resolve    { any_ref: AnyEntityRef },
    Checkout   { any_ref: AnyEntityRef },
    Commit     { entity: TrackedEntity, any_ref: AnyEntityRef },
    Remove     { any_ref: AnyEntityRef },        // returns TrackedEntity so caller can undo
    Persist,
    Load       { any_ref: AnyEntityRef, fields: Vec<String> },
    UndoCommit { any_ref: AnyEntityRef },
    Unload     { any_ref: AnyEntityRef },
}

enum StoreCommand {
    Insert(TrackedEntity),                   // AnyEntityRef extracted from entity internally
    UndoCheckout { any_ref: AnyEntityRef },  // release lock, drop changes
}

enum StoreResponse {
    Entity(TrackedEntity),
    Unit,
}

enum StoreMessage {
    Request { request: StoreRequest, reply: oneshot::Sender<Result<StoreResponse, StoreError>> },
    Command(StoreCommand),
}
```

`StoreRequest` variants require a response (round-trip). `StoreCommand` variants are fire-and-forget — no response channel.

---

## Actor Loop

Thin dispatcher — delegates to `Store<S>` methods, no logic in match arms.

```rust
impl<S: Substrate> Store<S> {
    async fn run(mut self, mut rx: mpsc::Receiver<StoreMessage>) {
        while let Some(msg) = rx.recv().await {
            match msg {
                StoreMessage::Request { request, reply } => {
                    let _ = reply.send(self.handle(request).await);
                }
                StoreMessage::Command(cmd) => {
                    self.execute(cmd);
                }
            }
        }
    }
}
```

---

## Failure Model

If the actor task panics, `rx` is dropped. `EntityServer::sender().send()` returns `SendError` → surfaced as `StoreError::Unavailable`. No cascading panics.
