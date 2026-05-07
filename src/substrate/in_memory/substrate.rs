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

    fn projected_validator_for(
        kind: crate::entity::EntityKind,
        asset_path: &'static str,
    ) -> &'static std::sync::Arc<jsonschema::Validator> {
        static VALIDATORS: std::sync::LazyLock<
            std::collections::HashMap<
                (crate::entity::EntityKind, &'static str),
                std::sync::Arc<jsonschema::Validator>,
            >,
        > = std::sync::LazyLock::new(
            crate::substrate::lib::schema_registry::build_validators_for::<InMemorySubstrate>,
        );
        VALIDATORS
            .get(&(kind, asset_path))
            .expect("projected validator missing for (kind, asset_path)")
    }
}
