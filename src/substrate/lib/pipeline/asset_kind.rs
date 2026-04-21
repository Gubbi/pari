use super::AssetOp;

pub struct AssetKind {
    pub distinguishes_create: bool,
    pub supports_partial: bool,
}

impl AssetKind {
    pub fn write_op<E>(&self, is_create: bool, is_partial: bool, encoded: E) -> AssetOp<E> {
        if is_partial && self.supports_partial {
            AssetOp::Patch(encoded)
        } else if is_create && self.distinguishes_create {
            AssetOp::Post(encoded)
        } else {
            AssetOp::Put(encoded)
        }
    }
}
