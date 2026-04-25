/// Marker for backend-specific slot kinds used in `FieldMapping`
/// entries. Slots let a backend attach positional or structural
/// information to each field — the default `ValueSlot` covers backends
/// whose assets carry one value per field.
pub trait Slot: Copy + 'static {}

/// Default single-slot marker used by every backend that does not need
/// slot specialisation.
#[derive(Clone, Copy)]
pub enum ValueSlot {
    Value,
}

impl Slot for ValueSlot {}
