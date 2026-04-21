use crate::substrate::{
    in_memory::lib::{
        codec::InMemoryCodec, executor::InMemoryExecutor, resolver::InMemoryResolver,
        storage::InMemoryStorage,
    },
    pipeline::ValueSlot,
    Substrate,
};

pub struct InMemorySubstrate {
    resolver: InMemoryResolver,
    codec: InMemoryCodec,
    executor: InMemoryExecutor,
}

impl InMemorySubstrate {
    pub fn new() -> Self {
        Self::with_storage(InMemoryStorage::new())
    }

    pub fn with_storage(storage: InMemoryStorage) -> Self {
        Self {
            resolver: InMemoryResolver,
            codec: InMemoryCodec,
            executor: InMemoryExecutor::new(storage),
        }
    }
}

impl Default for InMemorySubstrate {
    fn default() -> Self {
        Self::new()
    }
}

impl Substrate for InMemorySubstrate {
    type Slot = ValueSlot;
    type Location = String;
    type Encoded = String;
    type Resolver = InMemoryResolver;
    type Codec = InMemoryCodec;
    type Executor = InMemoryExecutor;

    fn substrate_name() -> &'static str {
        "in_memory_substrate"
    }

    fn resolver(&self) -> &Self::Resolver {
        &self.resolver
    }

    fn codec(&self) -> &Self::Codec {
        &self.codec
    }

    fn executor(&self) -> &Self::Executor {
        &self.executor
    }
}
