//! `PrimitiveDetail` — erased name/value view of a primitive's typed fields.
//!
//! Primitive errors declare their own typed detail fields (e.g. `path: String`,
//! `line: usize`). For OTel emission and for generic inspection, those fields
//! also need to be available in a uniform, iterable shape. `PrimitiveDetail`
//! is that uniform shape.
//!
//! Each primitive error exposes `details() -> &[PrimitiveDetail]`, which the
//! OTel emitter iterates to produce structured fields under the
//! `error.<error_type>.<field_name>` namespace. Values are rendered as strings
//! so that heterogeneous field types fit a single slice.

/// Erased key/value representation of one typed field on a primitive error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrimitiveDetail {
    pub field_name: &'static str,
    pub value: String,
}
