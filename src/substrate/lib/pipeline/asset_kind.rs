use super::AssetOp;

/// Per-asset capability flags that drive write-op selection.
///
/// `distinguishes_create` — executor differentiates `Post` (create)
/// from `Put` (replace). Set on asset kinds whose backend
/// representation needs that distinction (e.g. HTTP-style create
/// semantics).
///
/// `supports_partial` — executor accepts `Patch` for partial writes.
/// When `false`, any mutation that does not cover every field in the
/// asset is upgraded to a full rewrite.
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
