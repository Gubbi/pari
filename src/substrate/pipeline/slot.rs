pub trait Slot: Copy + 'static {}

#[derive(Clone, Copy)]
pub enum ValueSlot {
    Value,
}

impl Slot for ValueSlot {}
